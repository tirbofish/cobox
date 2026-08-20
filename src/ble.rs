use std::sync::{
    mpsc::{sync_channel, Receiver, SyncSender, TrySendError},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

use enumset::enum_set;
use esp_idf_svc::bt::ble::gap::{
    AdvConfiguration, AuthenticationRequest, BleEncryption, BleGapEvent, EspBleGap, IOCapabilities,
    SecurityConfiguration,
};
use esp_idf_svc::bt::ble::gatt::server::{ConnectionId, EspGatts, GattsEvent, TransferId};
use esp_idf_svc::bt::ble::gatt::{
    set_local_mtu, AutoResponse, GattCharacteristic, GattId, GattInterface, GattResponse,
    GattServiceId, GattStatus, Handle, Permission, Property,
};
use esp_idf_svc::bt::{BdAddr, Ble, BtDriver, BtStatus, BtUuid};
use esp_idf_svc::hal::modem::BluetoothModemPeripheral;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::EspError;

use crate::blob::BlobConfig;

pub const SERVICE_UUID: u128 = 0xe902_8b60_37f3_4c25_b960_6af1_e715_0001;
pub const SETTINGS_UUID: u128 = 0xe902_8b60_37f3_4c25_b960_6af1_e715_0002;

const APP_ID: u16 = 0;
const DEVICE_NAME: &str = "Cobox";
const PAIRING_WINDOW: Duration = Duration::from_secs(120);
const LOCAL_MTU: u16 = 128;
const ROLL_STATS: u8 = 1;
const ROLL_LOOK: u8 = 2;
const MAX_SETUP_ROLLS: u8 = 3;

type CoboxBtDriver = BtDriver<'static, Ble>;
type CoboxGap = Arc<EspBleGap<'static, Ble, Arc<CoboxBtDriver>>>;
type CoboxGatts = Arc<EspGatts<'static, Ble, Arc<CoboxBtDriver>>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PairingResult {
    Succeeded,
    Failed,
}

pub struct BleManager {
    server: Arc<BleServer>,
    updates: Receiver<BlobConfig>,
    pairing_results: Receiver<PairingResult>,
    deadline: Option<Instant>,
}

impl BleManager {
    pub fn new(
        modem: impl BluetoothModemPeripheral + 'static,
        nvs: EspDefaultNvsPartition,
        config: BlobConfig,
    ) -> Result<Self, EspError> {
        let driver = Arc::new(BtDriver::new(modem, Some(nvs))?);
        set_local_mtu(LOCAL_MTU)?;

        let (updates_tx, updates) = sync_channel(1);
        let (pairing_results_tx, pairing_results) = sync_channel(2);
        let server = Arc::new(BleServer::new(
            Arc::new(EspBleGap::new(driver.clone())?),
            Arc::new(EspGatts::new(driver)?),
            config,
            updates_tx,
            pairing_results_tx,
        ));

        let gap_server = server.clone();
        server.gap.subscribe(move |event| {
            if let Err(error) = gap_server.on_gap_event(event) {
                log::warn!("BLE GAP operation failed: {error:?}");
            }
        })?;

        let gatts_server = server.clone();
        server.gatts.subscribe(move |(gatt_if, event)| {
            if let Err(error) = gatts_server.on_gatts_event(gatt_if, event) {
                log::warn!("BLE GATT operation failed: {error:?}");
            }
        })?;

        server.gatts.register_app(APP_ID)?;
        log::info!("BLE setup service initialized");

        Ok(Self {
            server,
            updates,
            pairing_results,
            deadline: None,
        })
    }

    pub fn begin_pairing(&mut self) -> Result<u32, EspError> {
        let passkey = crate::blob::random_u32() % 1_000_000;
        self.server.set_security(passkey)?;
        self.server.begin_window()?;
        self.deadline = Some(Instant::now() + PAIRING_WINDOW);
        log::info!("BLE pairing window opened");
        Ok(passkey)
    }

    pub fn pairing_active(&self) -> bool {
        self.deadline.is_some() && self.server.window_open()
    }

    pub fn expire_pairing_window(&mut self) -> Result<bool, EspError> {
        if self
            .deadline
            .is_some_and(|deadline| deadline <= Instant::now())
        {
            self.deadline = None;
            self.server.end_window()?;
            log::info!("BLE pairing window closed");
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn next_update(&self) -> Option<BlobConfig> {
        self.updates.try_recv().ok()
    }

    pub fn next_pairing_result(&mut self) -> Option<PairingResult> {
        let result = self.pairing_results.try_recv().ok()?;
        self.deadline = None;
        Some(result)
    }

    pub fn set_config(&self, config: BlobConfig) {
        self.server.state.lock().unwrap().config = config;
    }
}

struct BleServer {
    gap: CoboxGap,
    gatts: CoboxGatts,
    state: Mutex<State>,
    updates: SyncSender<BlobConfig>,
    pairing_results: SyncSender<PairingResult>,
}

struct State {
    service_handle: Option<Handle>,
    settings_handle: Option<Handle>,
    config: BlobConfig,
    window_open: bool,
    setup_rolls: u8,
    advertising: bool,
    advertising_configuring: bool,
    connection: Option<(ConnectionId, BdAddr)>,
    authenticated: Option<BdAddr>,
}

impl BleServer {
    fn new(
        gap: CoboxGap,
        gatts: CoboxGatts,
        config: BlobConfig,
        updates: SyncSender<BlobConfig>,
        pairing_results: SyncSender<PairingResult>,
    ) -> Self {
        Self {
            gap,
            gatts,
            state: Mutex::new(State {
                service_handle: None,
                settings_handle: None,
                config,
                window_open: false,
                setup_rolls: 0,
                advertising: false,
                advertising_configuring: false,
                connection: None,
                authenticated: None,
            }),
            updates,
            pairing_results,
        }
    }

    fn set_security(&self, passkey: u32) -> Result<(), EspError> {
        self.gap.set_security_conf(&SecurityConfiguration {
            auth_req_mode: AuthenticationRequest::MitmBonding,
            io_capabilities: IOCapabilities::DisplayOnly,
            static_passkey: Some(passkey),
            only_accept_specified_auth: true,
            ..Default::default()
        })
    }

    fn begin_window(&self) -> Result<(), EspError> {
        let (stop_advertising, disconnect) = {
            let mut state = self.state.lock().unwrap();
            state.window_open = true;
            state.authenticated = None;
            state.setup_rolls = 0;
            (state.advertising, state.connection.map(|(_, addr)| addr))
        };

        if stop_advertising {
            self.gap.stop_advertising()?;
        }
        if let Some(addr) = disconnect {
            self.gap.disconnect(addr)?;
        }
        self.configure_advertising_if_ready()
    }

    fn end_window(&self) -> Result<(), EspError> {
        let (stop_advertising, disconnect) = {
            let mut state = self.state.lock().unwrap();
            state.window_open = false;
            (
                state.advertising,
                state
                    .connection
                    .filter(|(_, addr)| state.authenticated != Some(*addr))
                    .map(|(_, addr)| addr),
            )
        };

        if stop_advertising {
            self.gap.stop_advertising()?;
        }
        if let Some(addr) = disconnect {
            self.gap.disconnect(addr)?;
        }
        Ok(())
    }

    fn window_open(&self) -> bool {
        self.state.lock().unwrap().window_open
    }

    fn configure_advertising_if_ready(&self) -> Result<(), EspError> {
        let configure = {
            let mut state = self.state.lock().unwrap();
            if state.window_open
                && state.settings_handle.is_some()
                && state.connection.is_none()
                && !state.advertising
                && !state.advertising_configuring
            {
                state.advertising_configuring = true;
                true
            } else {
                false
            }
        };

        if configure {
            self.gap.set_adv_conf(&AdvConfiguration {
                include_name: true,
                flag: 2,
                service_uuid: Some(BtUuid::uuid128(SERVICE_UUID)),
                ..Default::default()
            })?;
        }
        Ok(())
    }

    fn on_gap_event(&self, event: BleGapEvent) -> Result<(), EspError> {
        match event {
            BleGapEvent::AdvertisingConfigured(status) => {
                let start = {
                    let mut state = self.state.lock().unwrap();
                    state.advertising_configuring = false;
                    if status == BtStatus::Success
                        && state.window_open
                        && state.connection.is_none()
                    {
                        state.advertising = true;
                        true
                    } else {
                        false
                    }
                };
                if status != BtStatus::Success {
                    log::warn!("BLE advertising configuration failed: {status:?}");
                } else if start {
                    self.gap.start_advertising()?;
                }
            }
            BleGapEvent::AdvertisingStarted(status) => {
                if status != BtStatus::Success {
                    self.state.lock().unwrap().advertising = false;
                    log::warn!("BLE advertising failed to start: {status:?}");
                }
            }
            BleGapEvent::AdvertisingStopped(status) => {
                self.state.lock().unwrap().advertising = false;
                if status != BtStatus::Success {
                    log::warn!("BLE advertising failed to stop: {status:?}");
                } else {
                    self.configure_advertising_if_ready()?;
                }
            }
            BleGapEvent::AuthenticationComplete { bd_addr, status } => {
                self.authentication_complete(bd_addr, status)?;
            }
            BleGapEvent::PasskeyNotification { addr, .. } => {
                log::info!("BLE passkey notification for {addr}");
            }
            _ => {}
        }
        Ok(())
    }

    fn authentication_complete(&self, addr: BdAddr, status: BtStatus) -> Result<(), EspError> {
        let success = {
            let mut state = self.state.lock().unwrap();
            let active_connection = state
                .connection
                .is_some_and(|(_, connected_addr)| connected_addr == addr);
            if status == BtStatus::Success && state.window_open && active_connection {
                state.authenticated = Some(addr);
                state.window_open = false;
                true
            } else {
                state.authenticated = None;
                state.window_open = false;
                false
            }
        };

        if success {
            self.send_pairing_result(PairingResult::Succeeded);
            log::info!("BLE peer {addr} authenticated");
        } else {
            self.send_pairing_result(PairingResult::Failed);
            log::warn!("BLE authentication failed for {addr}: {status:?}");
            self.gap.disconnect(addr)?;
        }
        Ok(())
    }

    fn on_gatts_event(&self, gatt_if: GattInterface, event: GattsEvent) -> Result<(), EspError> {
        match event {
            GattsEvent::ServiceRegistered { status, app_id } => {
                if status != GattStatus::Ok {
                    log::warn!("BLE service registration failed: {status:?}");
                } else if app_id == APP_ID {
                    self.create_service(gatt_if)?;
                }
            }
            GattsEvent::ServiceCreated {
                status,
                service_handle,
                ..
            } => {
                if status != GattStatus::Ok {
                    log::warn!("BLE service creation failed: {status:?}");
                } else {
                    self.start_service(service_handle)?;
                }
            }
            GattsEvent::CharacteristicAdded {
                status,
                attr_handle,
                service_handle,
                char_uuid,
            } => {
                if status != GattStatus::Ok {
                    log::warn!("BLE settings characteristic creation failed: {status:?}");
                } else {
                    self.register_characteristic(service_handle, attr_handle, char_uuid)?;
                }
            }
            GattsEvent::PeerConnected { conn_id, addr, .. } => {
                self.peer_connected(conn_id, addr)?;
            }
            GattsEvent::PeerDisconnected { conn_id, addr, .. } => {
                self.peer_disconnected(conn_id, addr)?;
            }
            GattsEvent::Read {
                conn_id,
                trans_id,
                addr,
                handle,
                offset,
                is_long,
                need_rsp,
            } => {
                self.read(
                    gatt_if, conn_id, trans_id, addr, handle, offset, is_long, need_rsp,
                )?;
            }
            GattsEvent::Write {
                conn_id,
                trans_id,
                addr,
                handle,
                offset,
                need_rsp,
                is_prep,
                value,
            } => {
                self.write(
                    gatt_if, conn_id, trans_id, addr, handle, offset, need_rsp, is_prep, value,
                )?;
            }
            GattsEvent::ExecWrite {
                conn_id, trans_id, ..
            } => {
                log::warn!("rejected prepared BLE settings write");
                self.gatts.send_response(
                    gatt_if,
                    conn_id,
                    trans_id,
                    GattStatus::ReqNotSupported,
                    None,
                )?;
            }
            _ => {}
        }
        Ok(())
    }

    fn create_service(&self, gatt_if: GattInterface) -> Result<(), EspError> {
        self.gap.set_device_name(DEVICE_NAME)?;
        self.gatts.create_service(
            gatt_if,
            &GattServiceId {
                id: GattId {
                    uuid: BtUuid::uuid128(SERVICE_UUID),
                    inst_id: 0,
                },
                is_primary: true,
            },
            4,
        )
    }

    fn start_service(&self, service_handle: Handle) -> Result<(), EspError> {
        self.state.lock().unwrap().service_handle = Some(service_handle);
        self.gatts.start_service(service_handle)?;
        self.gatts.add_characteristic(
            service_handle,
            &GattCharacteristic {
                uuid: BtUuid::uuid128(SETTINGS_UUID),
                permissions: enum_set!(
                    Permission::ReadEncryptedMitm | Permission::WriteEncryptedMitm
                ),
                properties: enum_set!(Property::Read | Property::Write),
                max_len: BlobConfig::SERIALIZED_LEN,
                auto_rsp: AutoResponse::ByApp,
            },
            &[],
        )
    }

    fn register_characteristic(
        &self,
        service_handle: Handle,
        attr_handle: Handle,
        char_uuid: BtUuid,
    ) -> Result<(), EspError> {
        let settings = {
            let mut state = self.state.lock().unwrap();
            if state.service_handle == Some(service_handle)
                && char_uuid == BtUuid::uuid128(SETTINGS_UUID)
            {
                state.settings_handle = Some(attr_handle);
                true
            } else {
                false
            }
        };
        if settings {
            self.configure_advertising_if_ready()?;
        }
        Ok(())
    }

    fn peer_connected(&self, conn_id: ConnectionId, addr: BdAddr) -> Result<(), EspError> {
        let pairing = {
            let mut state = self.state.lock().unwrap();
            if state.window_open && state.connection.is_none() {
                state.connection = Some((conn_id, addr));
                state.advertising = false;
                true
            } else {
                false
            }
        };

        if !pairing {
            log::warn!("rejecting BLE peer {addr} outside pairing window");
            return self.gap.disconnect(addr);
        }

        if let Err(error) = self.gap.stop_advertising() {
            log::warn!("BLE advertising stop after peer connection failed: {error:?}");
        }
        self.gap
            .set_encryption(addr, BleEncryption::EncryptionMitm)?;
        log::info!("BLE peer {addr} connected; requested MITM encryption");
        Ok(())
    }

    fn peer_disconnected(&self, conn_id: ConnectionId, addr: BdAddr) -> Result<(), EspError> {
        {
            let mut state = self.state.lock().unwrap();
            if state
                .connection
                .is_some_and(|(connected_id, _)| connected_id == conn_id)
            {
                state.connection = None;
                state.authenticated = None;
            }
        }
        log::info!("BLE peer {addr} disconnected");
        self.configure_advertising_if_ready()
    }

    #[allow(clippy::too_many_arguments)]
    fn read(
        &self,
        gatt_if: GattInterface,
        conn_id: ConnectionId,
        trans_id: TransferId,
        addr: BdAddr,
        handle: Handle,
        offset: u16,
        _is_long: bool,
        need_rsp: bool,
    ) -> Result<(), EspError> {
        let (settings, authenticated, config) = {
            let state = self.state.lock().unwrap();
            (
                state.settings_handle == Some(handle),
                state.authenticated == Some(addr)
                    && state
                        .connection
                        .is_some_and(|(connected_id, _)| connected_id == conn_id),
                state.config,
            )
        };

        if !settings {
            return Ok(());
        }
        let offset = usize::from(offset);
        let status = if !authenticated {
            log::warn!("rejected unauthenticated BLE settings read from {addr}");
            GattStatus::InsufficientAuthentication
        } else if offset > BlobConfig::SERIALIZED_LEN {
            log::warn!("rejected BLE settings read beyond profile from {addr}");
            GattStatus::InvalidOffset
        } else {
            GattStatus::Ok
        };
        let bytes = config.serialize();
        self.send_response(
            gatt_if,
            conn_id,
            trans_id,
            handle,
            need_rsp,
            status,
            (status == GattStatus::Ok).then(|| (&bytes[offset..], offset as u16)),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn write(
        &self,
        gatt_if: GattInterface,
        conn_id: ConnectionId,
        trans_id: TransferId,
        addr: BdAddr,
        handle: Handle,
        offset: u16,
        need_rsp: bool,
        is_prep: bool,
        value: &[u8],
    ) -> Result<(), EspError> {
        let (settings, authenticated) = {
            let state = self.state.lock().unwrap();
            (
                state.settings_handle == Some(handle),
                state.authenticated == Some(addr)
                    && state
                        .connection
                        .is_some_and(|(connected_id, _)| connected_id == conn_id),
            )
        };
        if !settings {
            return Ok(());
        }

        let status = if !authenticated {
            log::warn!("rejected unauthenticated BLE settings write from {addr}");
            GattStatus::InsufficientAuthentication
        } else if offset != 0 {
            log::warn!("rejected partial BLE settings write from {addr}");
            GattStatus::InvalidOffset
        } else if is_prep {
            log::warn!("rejected prepared BLE settings write from {addr}");
            GattStatus::ReqNotSupported
        } else if value.len() == 1 {
            self.roll_setup(value[0])
        } else if value.len() != BlobConfig::SERIALIZED_LEN {
            log::warn!(
                "rejected BLE settings write with {} bytes from {addr}",
                value.len()
            );
            GattStatus::InvalidAttrLen
        } else {
            match BlobConfig::deserialize(value) {
                Ok(config) => match self.updates.try_send(config) {
                    Ok(()) => {
                        self.state.lock().unwrap().config = config;
                        GattStatus::Ok
                    }
                    Err(TrySendError::Full(_)) => {
                        log::warn!("rejected BLE settings write: update queue full");
                        GattStatus::InsufficientResource
                    }
                    Err(TrySendError::Disconnected(_)) => {
                        log::warn!("rejected BLE settings write: update receiver disconnected");
                        GattStatus::InsufficientResource
                    }
                },
                Err(error) => {
                    log::warn!("rejected malformed BLE settings write from {addr}: {error:?}");
                    GattStatus::InvalidPdu
                }
            }
        };

        self.send_response(gatt_if, conn_id, trans_id, handle, need_rsp, status, None)
    }

    fn roll_setup(&self, roll: u8) -> GattStatus {
        let next = {
            let state = self.state.lock().unwrap();
            if state.config.is_setup() || state.setup_rolls >= MAX_SETUP_ROLLS {
                return GattStatus::InvalidPdu;
            }
            match roll {
                ROLL_STATS => state
                    .config
                    .with_personality(crate::blob::Personality::random()),
                ROLL_LOOK => state.config.with_random_shape(),
                _ => return GattStatus::InvalidPdu,
            }
        };

        match self.updates.try_send(next) {
            Ok(()) => {
                let mut state = self.state.lock().unwrap();
                state.config = next;
                state.setup_rolls += 1;
                GattStatus::Ok
            }
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                log::warn!("rejected BLE setup roll: update queue unavailable");
                GattStatus::InsufficientResource
            }
        }
    }

    fn send_response(
        &self,
        gatt_if: GattInterface,
        conn_id: ConnectionId,
        trans_id: TransferId,
        handle: Handle,
        need_rsp: bool,
        status: GattStatus,
        value: Option<(&[u8], u16)>,
    ) -> Result<(), EspError> {
        if !need_rsp {
            return Ok(());
        }

        let mut response = GattResponse::default();
        let response = if let Some((value, offset)) = value {
            response.attr_handle(handle).offset(offset).value(value)?;
            Some(&response)
        } else {
            None
        };
        self.gatts
            .send_response(gatt_if, conn_id, trans_id, status, response)
    }

    fn send_pairing_result(&self, result: PairingResult) {
        if let Err(error) = self.pairing_results.try_send(result) {
            log::warn!("BLE pairing result queue unavailable: {error:?}");
        }
    }
}
