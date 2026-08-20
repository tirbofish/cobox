use std::mem;

use esp_idf_svc::sys;
use wasmi::{
    Caller, Config, Engine, ExternType, Linker, Module, Store, StoreLimits, StoreLimitsBuilder,
    TypedFunc,
};

pub const ABI_VERSION: u16 = 1;
pub const MAX_MODULE_BYTES: usize = 128 * 1024;

const HEADER_LEN: usize = 16;
const MAGIC: [u8; 4] = *b"CBXW";
const MAX_MEMORY_BYTES: usize = 64 * 1024;
const FUEL_PER_CALL: u64 = 20_000;
const HOST_MODULE: &str = "cobox";
const PARTITION_LABEL: &[u8] = b"plugins\0";

pub struct PluginStore;

impl PluginStore {
    pub fn load_active() -> Result<Option<Vec<u8>>, PluginStoreError> {
        // SAFETY: the label is NUL-terminated and the returned pointer is valid for the app lifetime.
        let partition = unsafe {
            sys::esp_partition_find_first(
                sys::esp_partition_type_t_ESP_PARTITION_TYPE_DATA,
                sys::esp_partition_subtype_t_ESP_PARTITION_SUBTYPE_ANY,
                PARTITION_LABEL.as_ptr().cast(),
            )
        };
        if partition.is_null() {
            return Err(PluginStoreError::PartitionMissing);
        }

        // SAFETY: esp_partition_find_first returned a non-null partition pointer.
        let partition_size = unsafe { (*partition).size as usize };
        if partition_size < HEADER_LEN {
            return Err(PluginStoreError::PartitionTooSmall);
        }

        let mut header = [0u8; HEADER_LEN];
        read_partition(partition, 0, &mut header)?;
        if header.iter().all(|byte| *byte == u8::MAX) {
            return Ok(None);
        }
        if header[..4] != MAGIC {
            return Err(PluginStoreError::BadMagic);
        }

        let abi = u16::from_le_bytes([header[4], header[5]]);
        if abi != ABI_VERSION {
            return Err(PluginStoreError::AbiMismatch(abi));
        }
        if u16::from_le_bytes([header[6], header[7]]) != 0 {
            return Err(PluginStoreError::ReservedBits);
        }

        let payload_len =
            u32::from_le_bytes([header[8], header[9], header[10], header[11]]) as usize;
        if payload_len == 0 {
            return Err(PluginStoreError::EmptyPayload);
        }
        if payload_len > MAX_MODULE_BYTES {
            return Err(PluginStoreError::Oversized(payload_len));
        }
        if HEADER_LEN
            .checked_add(payload_len)
            .filter(|end| *end <= partition_size)
            .is_none()
        {
            return Err(PluginStoreError::PayloadExceedsPartition);
        }

        let mut payload = Vec::new();
        payload
            .try_reserve_exact(payload_len)
            .map_err(|_| PluginStoreError::AllocationFailed)?;
        payload.resize(payload_len, 0);
        read_partition(partition, HEADER_LEN, &mut payload)?;

        let expected_crc = u32::from_le_bytes([header[12], header[13], header[14], header[15]]);
        if crc32(&payload) != expected_crc {
            return Err(PluginStoreError::CrcMismatch);
        }
        Ok(Some(payload))
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum PluginStoreError {
    PartitionMissing,
    PartitionTooSmall,
    ReadFailed(i32),
    BadMagic,
    AbiMismatch(u16),
    ReservedBits,
    EmptyPayload,
    Oversized(usize),
    PayloadExceedsPartition,
    AllocationFailed,
    CrcMismatch,
}

fn read_partition(
    partition: *const sys::esp_partition_t,
    offset: usize,
    bytes: &mut [u8],
) -> Result<(), PluginStoreError> {
    // SAFETY: `partition` comes from ESP-IDF, and `bytes` is a valid writable buffer.
    let result = unsafe {
        sys::esp_partition_read(partition, offset, bytes.as_mut_ptr().cast(), bytes.len())
    };
    if result == sys::ESP_OK {
        Ok(())
    } else {
        Err(PluginStoreError::ReadFailed(result))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Expression {
    pub bob_offset: i32,
    pub eye_scale: i32,
}

#[derive(Default)]
pub struct PluginActions {
    pub led: Option<[u8; 3]>,
    pub expression: Option<Expression>,
}

struct HostState {
    actions: PluginActions,
    limits: StoreLimits,
}

pub struct PluginRuntime {
    store: Store<HostState>,
    init: TypedFunc<(), i32>,
    tick: TypedFunc<i32, i32>,
}

impl PluginRuntime {
    pub fn load(bytes: &[u8]) -> Result<(Self, PluginActions), PluginRuntimeError> {
        let mut config = Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config);
        let module = Module::new(&engine, &mut &bytes[..]).map_err(PluginRuntimeError::Wasmi)?;
        validate_memory_limits(bytes)?;
        validate_imports(&module)?;

        let limits = StoreLimitsBuilder::new()
            .memory_size(MAX_MEMORY_BYTES)
            .memories(1)
            .instances(1)
            .tables(1)
            .table_elements(1024)
            .build();
        let mut store = Store::new(
            &engine,
            HostState {
                actions: PluginActions::default(),
                limits,
            },
        );
        store.limiter(|state| &mut state.limits);

        let mut linker = Linker::new(&engine);
        linker
            .func_wrap(
                HOST_MODULE,
                "cobox_set_led",
                |mut caller: Caller<'_, HostState>, red: i32, green: i32, blue: i32| {
                    caller.data_mut().actions.led = Some([
                        red.clamp(0, 255) as u8,
                        green.clamp(0, 255) as u8,
                        blue.clamp(0, 255) as u8,
                    ]);
                },
            )
            .map_err(|error| PluginRuntimeError::Wasmi(error.into()))?;
        linker
            .func_wrap(
                HOST_MODULE,
                "cobox_set_expression",
                |mut caller: Caller<'_, HostState>, bob_offset: i32, eye_scale: i32| {
                    caller.data_mut().actions.expression = Some(Expression {
                        bob_offset: bob_offset.clamp(-20, 20),
                        eye_scale: eye_scale.clamp(25, 200),
                    });
                },
            )
            .map_err(|error| PluginRuntimeError::Wasmi(error.into()))?;

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(PluginRuntimeError::Wasmi)?
            .start(&mut store)
            .map_err(PluginRuntimeError::Wasmi)?;
        let init = instance
            .get_typed_func::<(), i32>(&store, "cobox_init")
            .map_err(PluginRuntimeError::Wasmi)?;
        let tick = instance
            .get_typed_func::<i32, i32>(&store, "cobox_tick")
            .map_err(PluginRuntimeError::Wasmi)?;

        let mut runtime = Self { store, init, tick };
        runtime.call_init()?;
        let actions = mem::take(&mut runtime.store.data_mut().actions);
        Ok((runtime, actions))
    }

    pub fn tick(&mut self, now_ms: i32) -> Result<PluginActions, PluginTickError> {
        self.store.data_mut().actions = PluginActions::default();
        let fuel_before = add_fuel(&mut self.store);
        let result = self.tick.call(&mut self.store, now_ms);
        drain_fuel(&mut self.store, fuel_before);
        match result {
            Ok(0) => Ok(mem::take(&mut self.store.data_mut().actions)),
            Ok(code) => Err(PluginTickError::Failed(code)),
            Err(trap) => Err(PluginTickError::Trap(trap)),
        }
    }

    fn call_init(&mut self) -> Result<(), PluginRuntimeError> {
        let fuel_before = add_fuel(&mut self.store);
        let result = self.init.call(&mut self.store, ());
        drain_fuel(&mut self.store, fuel_before);
        match result {
            Ok(0) => Ok(()),
            Ok(code) => Err(PluginRuntimeError::InitFailed(code)),
            Err(trap) => Err(PluginRuntimeError::InitTrap(trap)),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum PluginRuntimeError {
    ForbiddenImport,
    InvalidMemory,
    Wasmi(wasmi::Error),
    InitTrap(wasmi::core::Trap),
    InitFailed(i32),
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum PluginTickError {
    Trap(wasmi::core::Trap),
    Failed(i32),
}

fn validate_imports(module: &Module) -> Result<(), PluginRuntimeError> {
    for import in module.imports() {
        let allowed_name = matches!(import.name(), "cobox_set_led" | "cobox_set_expression");
        if import.module() != HOST_MODULE
            || !allowed_name
            || !matches!(import.ty(), ExternType::Func(_))
        {
            return Err(PluginRuntimeError::ForbiddenImport);
        }
    }
    Ok(())
}

fn validate_memory_limits(bytes: &[u8]) -> Result<(), PluginRuntimeError> {
    if bytes.get(..8) != Some(b"\0asm\x01\0\0\0") {
        return Err(PluginRuntimeError::InvalidMemory);
    }

    let mut offset = 8;
    let mut saw_memory_section = false;
    while offset < bytes.len() {
        let section_id = *bytes.get(offset).ok_or(PluginRuntimeError::InvalidMemory)?;
        offset += 1;
        let section_len = read_u32_leb(bytes, &mut offset)? as usize;
        let section_end = offset
            .checked_add(section_len)
            .filter(|end| *end <= bytes.len())
            .ok_or(PluginRuntimeError::InvalidMemory)?;

        if section_id == 5 {
            if saw_memory_section {
                return Err(PluginRuntimeError::InvalidMemory);
            }
            saw_memory_section = true;
            let count = read_u32_leb(bytes, &mut offset)?;
            if count > 1 {
                return Err(PluginRuntimeError::InvalidMemory);
            }
            if count == 1 {
                let flags = read_u32_leb(bytes, &mut offset)?;
                let initial = read_u32_leb(bytes, &mut offset)?;
                let maximum = if flags == 1 {
                    read_u32_leb(bytes, &mut offset)?
                } else {
                    return Err(PluginRuntimeError::InvalidMemory);
                };
                if initial > 1 || maximum > 1 || maximum < initial {
                    return Err(PluginRuntimeError::InvalidMemory);
                }
            }
            if offset != section_end {
                return Err(PluginRuntimeError::InvalidMemory);
            }
        }
        offset = section_end;
    }
    Ok(())
}

fn read_u32_leb(bytes: &[u8], offset: &mut usize) -> Result<u32, PluginRuntimeError> {
    let mut value = 0u64;
    for shift in (0..35).step_by(7) {
        let byte = *bytes
            .get(*offset)
            .ok_or(PluginRuntimeError::InvalidMemory)?;
        *offset += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return u32::try_from(value).map_err(|_| PluginRuntimeError::InvalidMemory);
        }
    }
    Err(PluginRuntimeError::InvalidMemory)
}

fn add_fuel(store: &mut Store<HostState>) -> u64 {
    let before = store.fuel_consumed().expect("fuel metering enabled");
    store
        .add_fuel(FUEL_PER_CALL)
        .expect("fuel metering enabled");
    before
}

fn drain_fuel(store: &mut Store<HostState>, before: u64) {
    let consumed = store
        .fuel_consumed()
        .expect("fuel metering enabled")
        .saturating_sub(before);
    store
        .consume_fuel(FUEL_PER_CALL.saturating_sub(consumed))
        .expect("fuel was reserved");
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}
