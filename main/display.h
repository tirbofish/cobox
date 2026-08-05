#pragma once

#include <stdbool.h>
#include <stdint.h>
#include "esp_err.h"
#include "lvgl.h"

#ifdef __cplusplus
extern "C" {
#endif

#define LCD_H_RES  240
#define LCD_V_RES  240

/** Init backlight, ST7789 SPI panel, and LVGL. */
esp_err_t display_init(void);

/** Lock before calling LVGL APIs from other tasks. */
bool display_lock(uint32_t timeout_ms);
void display_unlock(void);

lv_display_t *display_get(void);

#ifdef __cplusplus
}
#endif
