#include "st7735.h"

lv_display_t *st7735_create(lv_lcd_send_cmd_cb_t send_cmd, lv_lcd_send_color_cb_t send_color)
{
    lv_display_t *disp = lv_st7735_create(ST7735_H_RES, ST7735_V_RES,
                                          LV_LCD_FLAG_BGR | LV_LCD_FLAG_MIRROR_X | LV_LCD_FLAG_MIRROR_Y,
                                          send_cmd, send_color);
    if (disp) {
        lv_st7735_set_gap(disp, ST7735_X_GAP, ST7735_Y_GAP);
        lv_st7735_set_invert(disp, true);
    }
    return disp;
}
