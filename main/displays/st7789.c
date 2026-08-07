#include "st7789.h"

lv_display_t *st7789_create(lv_lcd_send_cmd_cb_t send_cmd, lv_lcd_send_color_cb_t send_color)
{
    lv_display_t *disp = lv_st7789_create(ST7789_H_RES, ST7789_V_RES, LV_LCD_FLAG_NONE, send_cmd, send_color);
    if (disp) {
        lv_st7789_set_invert(disp, true);
    }
    return disp;
}
