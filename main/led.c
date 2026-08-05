#include "led.h"

#include "esp_check.h"
#include "esp_log.h"
#include "driver/gpio.h"
#include "driver/ledc.h"

static const char *TAG = "cobox:led";

#define PIN_LED_R            GPIO_NUM_27
#define PIN_LED_G            GPIO_NUM_32
#define PIN_LED_B            GPIO_NUM_33

#define LEDC_MODE            LEDC_LOW_SPEED_MODE
#define LEDC_TIMER           LEDC_TIMER_0
#define LEDC_DUTY_RES        LEDC_TIMER_8_BIT
#define LEDC_FREQ_HZ         5000
#define LEDC_CH_R            LEDC_CHANNEL_0
#define LEDC_CH_G            LEDC_CHANNEL_1
#define LEDC_CH_B            LEDC_CHANNEL_2

/* 1 = common-cathode (HIGH = on). Set 0 for common-anode. */
#define LED_ACTIVE_HIGH      1

static esp_err_t led_set_channel(ledc_channel_t ch, uint8_t level)
{
#if !LED_ACTIVE_HIGH
    level = (uint8_t)(255 - level);
#endif
    ESP_RETURN_ON_ERROR(ledc_set_duty(LEDC_MODE, ch, level), TAG, "set_duty");
    ESP_RETURN_ON_ERROR(ledc_update_duty(LEDC_MODE, ch), TAG, "update_duty");
    return ESP_OK;
}

esp_err_t led_init(void)
{
    const ledc_timer_config_t timer = {
        .speed_mode = LEDC_MODE,
        .duty_resolution = LEDC_DUTY_RES,
        .timer_num = LEDC_TIMER,
        .freq_hz = LEDC_FREQ_HZ,
        .clk_cfg = LEDC_AUTO_CLK,
    };
    ESP_RETURN_ON_ERROR(ledc_timer_config(&timer), TAG, "timer config failed");

    const ledc_channel_config_t channels[] = {
        { .speed_mode = LEDC_MODE, .channel = LEDC_CH_R, .timer_sel = LEDC_TIMER,
          .intr_type = LEDC_INTR_DISABLE, .gpio_num = PIN_LED_R, .duty = 0, .hpoint = 0 },
        { .speed_mode = LEDC_MODE, .channel = LEDC_CH_G, .timer_sel = LEDC_TIMER,
          .intr_type = LEDC_INTR_DISABLE, .gpio_num = PIN_LED_G, .duty = 0, .hpoint = 0 },
        { .speed_mode = LEDC_MODE, .channel = LEDC_CH_B, .timer_sel = LEDC_TIMER,
          .intr_type = LEDC_INTR_DISABLE, .gpio_num = PIN_LED_B, .duty = 0, .hpoint = 0 },
    };

    for (size_t i = 0; i < sizeof(channels) / sizeof(channels[0]); i++) {
        ESP_RETURN_ON_ERROR(ledc_channel_config(&channels[i]), TAG, "channel config failed");
    }

    ESP_LOGI(TAG, "RGB LED ready (R=%d G=%d B=%d)", PIN_LED_R, PIN_LED_G, PIN_LED_B);
    return led_off();
}

esp_err_t led_set_rgb(uint8_t r, uint8_t g, uint8_t b)
{
    ESP_RETURN_ON_ERROR(led_set_channel(LEDC_CH_R, r), TAG, "R");
    ESP_RETURN_ON_ERROR(led_set_channel(LEDC_CH_G, g), TAG, "G");
    ESP_RETURN_ON_ERROR(led_set_channel(LEDC_CH_B, b), TAG, "B");
    return ESP_OK;
}

esp_err_t led_set_color(led_color_t color)
{
    switch (color) {
    case LED_COLOR_RED:     return led_set_rgb(255, 0, 0);
    case LED_COLOR_GREEN:   return led_set_rgb(0, 255, 0);
    case LED_COLOR_BLUE:    return led_set_rgb(0, 0, 255);
    case LED_COLOR_YELLOW:  return led_set_rgb(255, 255, 0);
    case LED_COLOR_CYAN:    return led_set_rgb(0, 255, 255);
    case LED_COLOR_MAGENTA: return led_set_rgb(255, 0, 255);
    case LED_COLOR_WHITE:   return led_set_rgb(255, 255, 255);
    case LED_COLOR_OFF:
    default:                return led_off();
    }
}

esp_err_t led_off(void)
{
    return led_set_rgb(0, 0, 0);
}

const char *led_color_name(led_color_t color)
{
    switch (color) {
    case LED_COLOR_RED:     return "Red";
    case LED_COLOR_GREEN:   return "Green";
    case LED_COLOR_BLUE:    return "Blue";
    case LED_COLOR_YELLOW:  return "Yellow";
    case LED_COLOR_CYAN:    return "Cyan";
    case LED_COLOR_MAGENTA: return "Magenta";
    case LED_COLOR_WHITE:   return "White";
    case LED_COLOR_OFF:
    default:                return "Off";
    }
}
