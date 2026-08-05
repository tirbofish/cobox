#pragma once

#include <stdint.h>
#include "esp_err.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    LED_COLOR_OFF = 0,
    LED_COLOR_RED,
    LED_COLOR_GREEN,
    LED_COLOR_BLUE,
    LED_COLOR_YELLOW,
    LED_COLOR_CYAN,
    LED_COLOR_MAGENTA,
    LED_COLOR_WHITE,
    LED_COLOR_COUNT,
} led_color_t;

esp_err_t led_init(void);

/** Set channel brightness 0–255. */
esp_err_t led_set_rgb(uint8_t r, uint8_t g, uint8_t b);

esp_err_t led_set_color(led_color_t color);
esp_err_t led_off(void);

const char *led_color_name(led_color_t color);

#ifdef __cplusplus
}
#endif
