#pragma once

#include "lvgl.h"

#ifdef __cplusplus
extern "C" {
#endif

#define ST7735_H_RES    128
#define ST7735_V_RES    160
#define ST7735_SPI_MODE 0
#define ST7735_PCLK_HZ  (26 * 1000 * 1000)
#define ST7735_X_GAP    0
#define ST7735_Y_GAP    0

lv_display_t *st7735_create(lv_lcd_send_cmd_cb_t send_cmd, lv_lcd_send_color_cb_t send_color);

#ifdef __cplusplus
}
#endif
