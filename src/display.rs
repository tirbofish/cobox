use core::fmt;

use embedded_graphics::framebuffer::{buffer_size, Framebuffer};
use embedded_graphics::image::Image;
use embedded_graphics::mono_font::ascii::{FONT_10X20, FONT_6X10};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::raw::{LittleEndian, RawU16};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::Text;
use embedded_hal::spi::MODE_0;
use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::{
    AnyIOPin, Gpio15, Gpio16, Gpio18, Gpio23, Gpio4, Gpio5, GpioError, Output, PinDriver,
};
use esp_idf_svc::hal::spi::{
    config::Config, SpiDeviceDriver, SpiDriverConfig, SpiError as HalSpiError,
    SpiSingleDeviceDriver, SPI2,
};
use esp_idf_svc::hal::units::FromValueType;
use esp_idf_svc::sys::EspError;
use mipidsi::interface::{SpiError as MipiSpiError, SpiInterface};
use mipidsi::models::ST7735s;
use mipidsi::options::Orientation;
use mipidsi::{Builder, Display, InitError};

pub const BUFFER_SIZE: usize = 4096;
const SCREEN_WIDTH: usize = 128;
const SCREEN_HEIGHT: usize = 160;

type TextScreen = Framebuffer<
    Rgb565,
    RawU16,
    LittleEndian,
    SCREEN_WIDTH,
    SCREEN_HEIGHT,
    { buffer_size::<Rgb565>(SCREEN_WIDTH, SCREEN_HEIGHT) },
>;

pub type St7735Display<'d, 'buf> = Display<
    SpiInterface<'buf, SpiSingleDeviceDriver<'d>, PinDriver<'d, Output>>,
    ST7735s,
    PinDriver<'d, Output>,
>;

type St7735InitError = InitError<MipiSpiError<HalSpiError, GpioError>, GpioError>;

#[derive(Debug)]
pub enum DisplayInitError {
    Spi(EspError),
    Gpio(EspError),
    Init(St7735InitError),
}

impl fmt::Display for DisplayInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spi(error) => write!(f, "SPI setup failed: {error}"),
            Self::Gpio(error) => write!(f, "GPIO setup failed: {error}"),
            Self::Init(error) => write!(f, "display initialization failed: {error:?}"),
        }
    }
}

pub struct DisplayBundle<'d, 'buf> {
    pub display: St7735Display<'d, 'buf>,
    text: Box<TextScreen>,
    _backlight: PinDriver<'d, Output>,
}

pub fn init<'d, 'buf>(
    spi2: SPI2<'d>,
    sclk: Gpio18<'d>,
    sdo: Gpio23<'d>,
    cs: Gpio5<'d>,
    dc: Gpio16<'d>,
    rst: Gpio15<'d>,
    backlight: Gpio4<'d>,
    buffer: &'buf mut [u8],
    width: u16,
    height: u16,
) -> Result<DisplayBundle<'d, 'buf>, DisplayInitError> {
    let rst = PinDriver::output(rst).map_err(DisplayInitError::Gpio)?;
    let dc = PinDriver::output(dc).map_err(DisplayInitError::Gpio)?;
    let mut backlight = PinDriver::output(backlight).map_err(DisplayInitError::Gpio)?;

    let config = Config::new().baudrate(26.MHz().into()).data_mode(MODE_0);
    let spi = SpiDeviceDriver::new_single(
        spi2,
        sclk,
        sdo,
        None::<AnyIOPin>,
        Some(cs),
        &SpiDriverConfig::new(),
        &config,
    )
    .map_err(DisplayInitError::Spi)?;

    let di = SpiInterface::new(spi, dc, buffer);
    let mut delay = Ets;
    let display = Builder::new(ST7735s, di)
        .reset_pin(rst)
        .display_size(width, height)
        .orientation(Orientation::new())
        .init(&mut delay)
        .map_err(DisplayInitError::Init)?;

    backlight.set_high().map_err(DisplayInitError::Gpio)?;

    Ok(DisplayBundle {
        display,
        text: Box::new(Framebuffer::new()),
        _backlight: backlight,
    })
}

impl<'d, 'buf> DisplayBundle<'d, 'buf> {
    pub fn draw_setup(
        &mut self,
        show_qr: bool,
        paired: bool,
    ) -> Result<(), <St7735Display<'d, 'buf> as DrawTarget>::Error> {
        let label = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);
        self.text.clear(Rgb565::BLACK).unwrap();
        if show_qr {
            for (y, row) in SETUP_QR.iter().enumerate() {
                for (x, module) in row.bytes().enumerate() {
                    if module == b'#' {
                        Rectangle::new(
                            Point::new(22 + x as i32 * 4, 34 + y as i32 * 4),
                            Size::new(4, 4),
                        )
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
                        .draw(&mut *self.text)
                        .unwrap();
                    }
                }
            }
            Text::new("SCAN TO SET UP", Point::new(25, 130), label)
                .draw(&mut *self.text)
                .unwrap();
            Text::new("BACK: PAIR", Point::new(37, 145), label)
                .draw(&mut *self.text)
                .unwrap();
        } else if paired {
            Text::new("OPEN COBOX APP", Point::new(20, 52), label)
                .draw(&mut *self.text)
                .unwrap();
            Text::new("FINISH SETUP", Point::new(31, 76), label)
                .draw(&mut *self.text)
                .unwrap();
            Text::new("SAVE YOUR BLOB", Point::new(22, 108), label)
                .draw(&mut *self.text)
                .unwrap();
        } else {
            Text::new("DOWNLOAD THE", Point::new(25, 42), label)
                .draw(&mut *self.text)
                .unwrap();
            Text::new(
                "COBOX APP",
                Point::new(35, 62),
                MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE),
            )
            .draw(&mut *self.text)
            .unwrap();
            Text::new("SWITCH: QR CODE", Point::new(13, 105), label)
                .draw(&mut *self.text)
                .unwrap();
            Text::new("BACK: PAIR", Point::new(37, 130), label)
                .draw(&mut *self.text)
                .unwrap();
        }
        Image::new(&self.text.as_image(), Point::zero()).draw(&mut self.display)?;
        Ok(())
    }

    pub fn draw_pairing(
        &mut self,
        passkey: u32,
    ) -> Result<(), <St7735Display<'d, 'buf> as DrawTarget>::Error> {
        let label = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);
        let code = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
        let mut passkey_text = [b'0'; 6];
        let mut value = passkey;
        for digit in passkey_text.iter_mut().rev() {
            *digit += (value % 10) as u8;
            value /= 10;
        }
        let passkey = core::str::from_utf8(&passkey_text).unwrap();

        self.text.clear(Rgb565::BLACK).unwrap();
        Text::new("PAIR COBOX", Point::new(30, 35), label)
            .draw(&mut *self.text)
            .unwrap();
        Text::new("CODE", Point::new(50, 65), label)
            .draw(&mut *self.text)
            .unwrap();
        Text::new(&passkey, Point::new(34, 100), code)
            .draw(&mut *self.text)
            .unwrap();
        Text::new("120 SEC", Point::new(43, 130), label)
            .draw(&mut *self.text)
            .unwrap();
        Image::new(&self.text.as_image(), Point::zero()).draw(&mut self.display)?;
        Ok(())
    }
}

const SETUP_QR: [&str; 21] = [
    "#######..#.##.#######",
    "#.....#.##.#..#.....#",
    "#.###.#.##..#.#.###.#",
    "#.###.#..#.#..#.###.#",
    "#.###.#.#...#.#.###.#",
    "#.....#.#..##.#.....#",
    "#######.#.#.#.#######",
    "........#####........",
    "##.#..##.##...###.##.",
    "...##..##.##..###.###",
    "#.##.####.#.#....#..#",
    "######..##....#.##...",
    ".#.####.###.##.#....#",
    "........##..#..#.#..#",
    "#######.###.###.##.#.",
    "#.....#........##....",
    "#.###.#....###..#....",
    "#.###.#.##.##.#.#####",
    "#.###.#..#.####.##..#",
    "#.....#.#.##.#.##....",
    "#######.#.#.###....#.",
];
