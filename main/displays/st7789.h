#pragma once

#include "lvgl.h"

#ifdef __cplusplus
extern "C" {
#endif

#define ST7789_H_RES    240
#define ST7789_V_RES    240
#define ST7789_SPI_MODE 3
#define ST7789_PCLK_HZ  (26 * 1000 * 1000)

lv_display_t *st7789_create(lv_lcd_send_cmd_cb_t send_cmd, lv_lcd_send_color_cb_t send_color);

#ifdef __cplusplus
}
#endif
