#include "blob_gen.h"

#include "display.h"
#include "esp_random.h"
#include "lvgl.h"

static int rnd(int lo, int hi)
{
    return lo + (int)(esp_random() % (unsigned)(hi - lo + 1));
}

static void part(lv_obj_t *parent, int cx, int cy, int w, int h, lv_color_t color)
{
    lv_obj_t *o = lv_obj_create(parent);
    lv_obj_remove_style_all(o);
    lv_obj_set_size(o, w, h);
    lv_obj_set_pos(o, cx - w / 2, cy - h / 2);
    lv_obj_set_style_bg_color(o, color, 0);
    lv_obj_set_style_bg_opa(o, LV_OPA_COVER, 0);
    lv_obj_set_style_radius(o, LV_RADIUS_CIRCLE, 0);
    lv_obj_set_style_transform_pivot_x(o, w / 2, 0);
    lv_obj_set_style_transform_pivot_y(o, h / 2, 0);
    lv_obj_remove_flag(o, LV_OBJ_FLAG_CLICKABLE | LV_OBJ_FLAG_SCROLLABLE);
}

static void breathe_cb(void *var, int32_t v)
{
    lv_obj_set_style_transform_scale(var, v, 0);
}

static void blink_cb(void *var, int32_t v)
{
    lv_obj_t *blob = var;
    uint32_t n = lv_obj_get_child_count(blob);
    lv_obj_set_style_transform_scale_y(lv_obj_get_child(blob, n - 2), v, 0);
    lv_obj_set_style_transform_scale_y(lv_obj_get_child(blob, n - 1), v, 0);
}

void generate_blob(void)
{
    if (!display_lock(0)) {
        return;
    }

    lv_obj_set_style_bg_color(lv_scr_act(), lv_color_hex(0x003a57), LV_PART_MAIN);

    lv_obj_t *blob = lv_obj_create(lv_scr_act());
    lv_obj_remove_style_all(blob);
    lv_obj_set_size(blob, LCD_H_RES, LCD_V_RES);
    lv_obj_remove_flag(blob, LV_OBJ_FLAG_CLICKABLE | LV_OBJ_FLAG_SCROLLABLE);
    lv_obj_set_style_transform_pivot_x(blob, LCD_H_RES / 2, 0);
    lv_obj_set_style_transform_pivot_y(blob, LCD_V_RES / 2, 0);

    const lv_color_t color = lv_color_hsv_to_rgb(rnd(0, 359), rnd(70, 100), rnd(80, 100));
    const int cx = LCD_H_RES / 2;
    const int cy = LCD_V_RES / 2;
    const uint8_t n = rnd(5, 7);

    int core = rnd(68, 84);
    part(blob, cx, cy, core, core, color);
    for (uint8_t i = 1; i < n; i++) {
        int deg = (int)(i - 1) * 360 / (n - 1) + rnd(-18, 18);
        int dist = rnd(6, 14);
        int x = cx + ((dist * lv_trigo_cos(deg)) >> LV_TRIGO_SHIFT);
        int y = cy + ((dist * lv_trigo_sin(deg)) >> LV_TRIGO_SHIFT);
        int d = rnd(40, 56);
        part(blob, x, y, d, d, color);
    }

    const lv_color_t eye = lv_color_make(0x10, 0x18, 0x20);
    part(blob, cx - 16, cy - 2, 10, 16, eye);
    part(blob, cx + 16, cy - 2, 10, 16, eye);

    lv_anim_t a;
    lv_anim_init(&a);
    lv_anim_set_var(&a, blob);
    lv_anim_set_exec_cb(&a, breathe_cb);
    lv_anim_set_values(&a, LV_SCALE_NONE - 8, LV_SCALE_NONE + 18);
    lv_anim_set_duration(&a, 1600);
    lv_anim_set_reverse_duration(&a, 1600);
    lv_anim_set_repeat_count(&a, LV_ANIM_REPEAT_INFINITE);
    lv_anim_set_path_cb(&a, lv_anim_path_ease_in_out);
    lv_anim_start(&a);

    lv_anim_init(&a);
    lv_anim_set_var(&a, blob);
    lv_anim_set_exec_cb(&a, blink_cb);
    lv_anim_set_values(&a, LV_SCALE_NONE, 32);
    lv_anim_set_duration(&a, 90);
    lv_anim_set_reverse_duration(&a, 90);
    lv_anim_set_delay(&a, 1800);
    lv_anim_set_repeat_delay(&a, 4000);
    lv_anim_set_repeat_count(&a, LV_ANIM_REPEAT_INFINITE);
    lv_anim_start(&a);

    display_unlock();
}
