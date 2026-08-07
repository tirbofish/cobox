#include "led.h"
#include "chips/board.h"

#include "esp_check.h"
#include "esp_log.h"
#include "driver/gpio.h"
#include "driver/ledc.h"

static const char *TAG = "cobox:led";

#define LEDC_MODE       LEDC_LOW_SPEED_MODE
#define LEDC_TIMER      LEDC_TIMER_0
#define LEDC_DUTY_RES   LEDC_TIMER_8_BIT
#define LEDC_FREQ_HZ    5000

/* 1 = common-cathode (HIGH = on). Set 0 for common-anode. */
#define LED_ACTIVE_HIGH 1

static esp_err_t led_set_channel(ledc_channel_t ch, uint8_t level)
{
#if !LED_ACTIVE_HIGH
    level = (uint8_t)(255 - level);
#endif
    ESP_RETURN_ON_ERROR(ledc_set_duty(LEDC_MODE, ch, level), TAG, "set_duty");
    return ledc_update_duty(LEDC_MODE, ch);
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

    const struct { ledc_channel_t ch; gpio_num_t pin; } chans[] = {
        { LEDC_CHANNEL_0, PIN_LED_R },
        { LEDC_CHANNEL_1, PIN_LED_G },
        { LEDC_CHANNEL_2, PIN_LED_B },
    };
    for (size_t i = 0; i < sizeof(chans) / sizeof(chans[0]); i++) {
        const ledc_channel_config_t cfg = {
            .speed_mode = LEDC_MODE,
            .channel = chans[i].ch,
            .timer_sel = LEDC_TIMER,
            .intr_type = LEDC_INTR_DISABLE,
            .gpio_num = chans[i].pin,
            .duty = 0,
            .hpoint = 0,
        };
        ESP_RETURN_ON_ERROR(ledc_channel_config(&cfg), TAG, "channel config failed");
    }

    return led_off();
}

esp_err_t led_set_rgb(uint8_t r, uint8_t g, uint8_t b)
{
    ESP_RETURN_ON_ERROR(led_set_channel(LEDC_CHANNEL_0, r), TAG, "R");
    ESP_RETURN_ON_ERROR(led_set_channel(LEDC_CHANNEL_1, g), TAG, "G");
    ESP_RETURN_ON_ERROR(led_set_channel(LEDC_CHANNEL_2, b), TAG, "B");
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
    static const char *names[] = {
        [LED_COLOR_OFF] = "Off",
        [LED_COLOR_RED] = "Red",
        [LED_COLOR_GREEN] = "Green",
        [LED_COLOR_BLUE] = "Blue",
        [LED_COLOR_YELLOW] = "Yellow",
        [LED_COLOR_CYAN] = "Cyan",
        [LED_COLOR_MAGENTA] = "Magenta",
        [LED_COLOR_WHITE] = "White",
    };
    return (color >= 0 && color < LED_COLOR_COUNT) ? names[color] : "Off";
}
