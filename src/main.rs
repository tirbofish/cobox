use std::thread;
use std::time::{Duration, Instant};

use esp_idf_svc::hal::gpio::{PinDriver, Pull};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;

mod ble;
mod blob;
mod button;
mod display;
mod led;
mod plugin;
mod storage;

const POLL: Duration = Duration::from_millis(20);
const PLUGIN_TICK: Duration = Duration::from_millis(250);

fn show_setup(display: &mut display::DisplayBundle<'_, '_>, show_qr: bool, paired: bool) {
    display.draw_setup(show_qr, paired).unwrap();
}

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    let booted_at = Instant::now();

    let p = Peripherals::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().expect("NVS init");
    let store = storage::BlobStore::new(nvs.clone()).expect("NVS storage init");

    let mut buffer = [0u8; display::BUFFER_SIZE];
    let mut display = display::init(
        p.spi2,
        p.pins.gpio18,
        p.pins.gpio23,
        p.pins.gpio5,
        p.pins.gpio16,
        p.pins.gpio15,
        p.pins.gpio4,
        &mut buffer,
        blob::WIDTH as u16,
        blob::HEIGHT as u16,
    )
    .expect("display init");
    let mut blob = match store.load().expect("load saved blob profile") {
        Some(config) => {
            log::info!("restored blob profile");
            blob::Blob::from_config(config)
        }
        None => {
            let blob = blob::Blob::generate();
            store.save(blob.config()).expect("save blob profile");
            log::info!("created blob profile");
            blob
        }
    };
    if blob.config().is_setup() {
        blob.draw(&mut display.display).unwrap();
    } else {
        show_setup(&mut display, false, false);
    }
    let mut ble = ble::BleManager::new(p.modem, nvs.clone(), blob.config()).expect("BLE init");

    let mut led = led::RgbLed::new(p.ledc, p.pins.gpio27, p.pins.gpio32, p.pins.gpio33).unwrap();
    led.set_rgb(0, 0, 0).unwrap();
    let (mut plugin, init_actions) = match plugin::PluginStore::load_active() {
        Ok(Some(module)) => match plugin::PluginRuntime::load(&module) {
            Ok((runtime, actions)) => {
                log::info!("plugin loaded");
                (Some(runtime), Some(actions))
            }
            Err(error) => {
                log::error!("plugin initialization failed; disabled: {error:?}");
                (None, None)
            }
        },
        Ok(None) => {
            log::info!("plugin store empty");
            (None, None)
        }
        Err(error) => {
            log::error!("plugin store rejected module: {error:?}");
            (None, None)
        }
    };
    if let Some(actions) = init_actions {
        if let Some([red, green, blue]) = actions.led {
            if let Err(error) = led.set_rgb(u32::from(red), u32::from(green), u32::from(blue)) {
                log::error!("plugin LED command failed: {error:?}");
            }
        }
        if let Some(expression) = actions.expression {
            blob.set_expression_overlay(expression.bob_offset, expression.eye_scale);
            if blob.config().is_setup() {
                blob.draw(&mut display.display).unwrap();
            }
        }
    }

    let pins = [
        PinDriver::input(p.pins.gpio19, Pull::Up).unwrap(),
        PinDriver::input(p.pins.gpio26, Pull::Up).unwrap(),
        PinDriver::input(p.pins.gpio25, Pull::Up).unwrap(),
    ];
    let mut btns = pins.each_ref().map(|pin| button::Button::new(pin.is_low()));
    const NAMES: [&str; 3] = ["Back", "Switch", "Select"];
    const COLORS: [(u32, u32, u32); 3] = [(255, 0, 0), (0, 255, 0), (0, 0, 255)];
    const BACK_BUTTON: usize = 0;
    const SWITCH_BUTTON: usize = 1;
    const SELECT_BUTTON: usize = 2;
    let mut last_plugin_tick = Instant::now();
    let mut show_setup_qr = false;
    let mut paired = false;

    log::info!("boot ready");

    loop {
        for i in 0..3 {
            if let Some(true) = btns[i].update(pins[i].is_low()) {
                let (rv, gv, bv) = COLORS[i];
                led.set_rgb(rv, gv, bv).unwrap();
                if i == BACK_BUTTON {
                    match ble.begin_pairing() {
                        Ok(passkey) => {
                            display.draw_pairing(passkey).unwrap();
                            log::info!("BLE setup started");
                        }
                        Err(error) => log::error!("BLE setup could not start: {error:?}"),
                    }
                } else if !blob.config().is_setup() {
                    show_setup_qr = !show_setup_qr;
                    if !ble.pairing_active() {
                        show_setup(&mut display, show_setup_qr, paired);
                    }
                } else if i == SWITCH_BUTTON {
                    blob.randomize_personality();
                    store.save(blob.config()).expect("save blob profile");
                    ble.set_config(blob.config());
                    if !ble.pairing_active() && blob.config().is_setup() {
                        blob.draw(&mut display.display).unwrap();
                    } else if !ble.pairing_active() {
                        show_setup(&mut display, show_setup_qr, paired);
                    }
                    log::info!("personality randomized");
                } else if i == SELECT_BUTTON {
                    blob.regenerate();
                    store.save(blob.config()).expect("save blob profile");
                    ble.set_config(blob.config());
                    if !ble.pairing_active() {
                        if blob.config().is_setup() {
                            blob.draw(&mut display.display).unwrap();
                        } else {
                            show_setup(&mut display, show_setup_qr, paired);
                        }
                    }
                }
                log::info!("{} pressed", NAMES[i]);
            }
        }
        while let Some(config) = ble.next_update() {
            store.save(config).expect("save BLE blob profile");
            blob = blob::Blob::from_config(config);
            ble.set_config(config);
            if !ble.pairing_active() && (blob.config().is_setup() || paired) {
                blob.draw(&mut display.display).unwrap();
            } else if !ble.pairing_active() {
                show_setup(&mut display, show_setup_qr, paired);
            }
            log::info!("applied BLE blob profile");
        }
        while let Some(result) = ble.next_pairing_result() {
            log::info!("BLE pairing {result:?}");
            paired = result == ble::PairingResult::Succeeded;
            if blob.config().is_setup() || paired {
                blob.draw(&mut display.display).unwrap();
            } else {
                show_setup(&mut display, show_setup_qr, paired);
            }
        }
        match ble.expire_pairing_window() {
            Ok(true) if blob.config().is_setup() => blob.draw(&mut display.display).unwrap(),
            Ok(true) => show_setup(&mut display, show_setup_qr, paired),
            Ok(false) => {}
            Err(error) => log::error!("BLE pairing window close failed: {error:?}"),
        }
        if last_plugin_tick.elapsed() >= PLUGIN_TICK {
            last_plugin_tick = Instant::now();
            if let Some(active_plugin) = plugin.as_mut() {
                let now_ms = i32::try_from(booted_at.elapsed().as_millis()).unwrap_or(i32::MAX);
                match active_plugin.tick(now_ms) {
                    Ok(actions) => {
                        if let Some([red, green, blue]) = actions.led {
                            if let Err(error) =
                                led.set_rgb(u32::from(red), u32::from(green), u32::from(blue))
                            {
                                log::error!("plugin LED command failed: {error:?}");
                            }
                        }
                        let expression_changed = match actions.expression {
                            Some(expression) => blob.set_expression_overlay(
                                expression.bob_offset,
                                expression.eye_scale,
                            ),
                            None => blob.clear_expression_overlay(),
                        };
                        if expression_changed && blob.config().is_setup() && !ble.pairing_active() {
                            blob.draw(&mut display.display).unwrap();
                        }
                    }
                    Err(error) => {
                        log::error!("plugin trapped or failed; disabled: {error:?}");
                        plugin = None;
                        if blob.clear_expression_overlay()
                            && blob.config().is_setup()
                            && !ble.pairing_active()
                        {
                            blob.draw(&mut display.display).unwrap();
                        }
                    }
                }
            }
        }
        if (blob.config().is_setup() || paired) && !ble.pairing_active() {
            blob.animate(&mut display.display).unwrap();
        }
        thread::sleep(POLL);
    }
}
