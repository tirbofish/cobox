use std::thread;
use std::time::Duration;

use esp_idf_svc::hal::gpio::{PinDriver, Pull};
use esp_idf_svc::hal::peripherals::Peripherals;

mod blob;
mod button;
mod display;
mod led;
mod storage;

const POLL: Duration = Duration::from_millis(20);

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let store = storage::BlobStore::new().expect("NVS init");
    let p = Peripherals::take().unwrap();

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
    blob.draw(&mut display.display).unwrap();

    let mut led = led::RgbLed::new(p.ledc, p.pins.gpio27, p.pins.gpio32, p.pins.gpio33).unwrap();
    led.set_rgb(0, 0, 0).unwrap();

    let pins = [
        PinDriver::input(p.pins.gpio19, Pull::Up).unwrap(),
        PinDriver::input(p.pins.gpio26, Pull::Up).unwrap(),
        PinDriver::input(p.pins.gpio25, Pull::Up).unwrap(),
    ];
    let mut btns = pins.each_ref().map(|pin| button::Button::new(pin.is_low()));
    const NAMES: [&str; 3] = ["Back", "Switch", "Select"];
    const COLORS: [(u32, u32, u32); 3] = [(255, 0, 0), (0, 255, 0), (0, 0, 255)];
    const SWITCH_BUTTON: usize = 1;
    const SELECT_BUTTON: usize = 2;

    log::info!("boot ready");

    loop {
        for i in 0..3 {
            if let Some(true) = btns[i].update(pins[i].is_low()) {
                let (rv, gv, bv) = COLORS[i];
                led.set_rgb(rv, gv, bv).unwrap();
                if i == SWITCH_BUTTON {
                    blob.randomize_personality();
                    store.save(blob.config()).expect("save blob profile");
                    blob.draw(&mut display.display).unwrap();
                    log::info!("personality randomized");
                } else if i == SELECT_BUTTON {
                    blob.regenerate();
                    store.save(blob.config()).expect("save blob profile");
                    blob.draw(&mut display.display).unwrap();
                }
                log::info!("{} pressed", NAMES[i]);
            }
        }
        blob.animate(&mut display.display).unwrap();
        thread::sleep(POLL);
    }
}
