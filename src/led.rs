use esp_idf_svc::hal::gpio::OutputPin;
use esp_idf_svc::hal::ledc::{config::TimerConfig, LedcDriver, LedcTimerDriver, LowSpeed, LEDC};
use esp_idf_svc::hal::units::FromValueType;
use esp_idf_svc::sys::EspError;

pub struct RgbLed<'d> {
    _timer: LedcTimerDriver<'d, LowSpeed>,
    red: LedcDriver<'d>,
    green: LedcDriver<'d>,
    blue: LedcDriver<'d>,
}

impl<'d> RgbLed<'d> {
    pub fn new(
        ledc: LEDC,
        red_pin: impl OutputPin + 'd,
        green_pin: impl OutputPin + 'd,
        blue_pin: impl OutputPin + 'd,
    ) -> Result<Self, EspError> {
        let timer = LedcTimerDriver::new(
            ledc.timer0,
            &TimerConfig::default().frequency(5.kHz().into()),
        )?;
        let red = LedcDriver::new(ledc.channel0, &timer, red_pin)?;
        let green = LedcDriver::new(ledc.channel1, &timer, green_pin)?;
        let blue = LedcDriver::new(ledc.channel2, &timer, blue_pin)?;

        Ok(Self {
            _timer: timer,
            red,
            green,
            blue,
        })
    }

    pub fn set_rgb(&mut self, red: u32, green: u32, blue: u32) -> Result<(), EspError> {
        self.red.set_duty(red)?;
        self.green.set_duty(green)?;
        self.blue.set_duty(blue)?;
        Ok(())
    }
}
