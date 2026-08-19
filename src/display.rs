use core::fmt;

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
        _backlight: backlight,
    })
}
