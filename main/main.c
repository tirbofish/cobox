#include <stdio.h>

#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "display.h"
#include "buttons.h"
#include "led.h"
#include "lvgl.h"

static const char *TAG = "cobox";

static struct {
    lv_obj_t *led_label;
    lv_obj_t *btn_labels[BTN_COUNT];
    led_color_t color;
    bool led_on;
} s_ui;

static void ui_refresh_led(void)
{
    if (s_ui.led_on) {
        led_set_color(s_ui.color);
    } else {
        led_off();
    }

    display_lock(0);
    char buf[48];
    snprintf(buf, sizeof(buf), "LED: %s", s_ui.led_on ? led_color_name(s_ui.color) : "Off");
    lv_label_set_text(s_ui.led_label, buf);
    display_unlock();
}

static void ui_refresh_button(button_id_t id, bool pressed)
{
    display_lock(0);
    char buf[32];
    snprintf(buf, sizeof(buf), "%s: %s", buttons_name(id), pressed ? "DOWN" : "up");
    lv_label_set_text(s_ui.btn_labels[id], buf);
    display_unlock();
}

static void on_button(const button_event_t *event, void *user_data)
{
    (void)user_data;
    ui_refresh_button(event->id, event->type == BTN_EVENT_PRESSED);
    if (event->type != BTN_EVENT_PRESSED) {
        return;
    }

    switch (event->id) {
    case BTN_SELECT:
        s_ui.color = (led_color_t)((s_ui.color % (LED_COLOR_COUNT - 1)) + 1); /* skip OFF */
        s_ui.led_on = true;
        ui_refresh_led();
        break;
    case BTN_SWITCH:
        s_ui.led_on = !s_ui.led_on;
        if (s_ui.led_on && s_ui.color == LED_COLOR_OFF) {
            s_ui.color = LED_COLOR_RED;
        }
        ui_refresh_led();
        break;
    case BTN_BACK:
        s_ui.led_on = false;
        ui_refresh_led();
        break;
    default:
        break;
    }
}

static void app_create_ui(void)
{
    display_lock(0);

    const int pad = (LCD_H_RES < 200) ? 4 : 12;
    const int row = (LCD_V_RES < 200) ? 16 : 28;
    const int left = (LCD_H_RES < 200) ? 6 : 24;

    lv_obj_t *scr = lv_screen_active();
    lv_obj_set_style_bg_color(scr, lv_color_hex(0x101820), 0);

    lv_obj_t *title = lv_label_create(scr);
    lv_label_set_text(title, "cobox");
    lv_obj_set_style_text_color(title, lv_color_hex(0xE8F1F8), 0);
    lv_obj_align(title, LV_ALIGN_TOP_MID, 0, pad);

    s_ui.led_label = lv_label_create(scr);
    lv_obj_set_style_text_color(s_ui.led_label, lv_color_hex(0x7CFFB2), 0);
    lv_obj_align(s_ui.led_label, LV_ALIGN_TOP_MID, 0, pad + row);

    for (int i = 0; i < BTN_COUNT; i++) {
        s_ui.btn_labels[i] = lv_label_create(scr);
        lv_obj_set_style_text_color(s_ui.btn_labels[i], lv_color_hex(0xC8D6E0), 0);
        lv_obj_align(s_ui.btn_labels[i], LV_ALIGN_LEFT_MID, left, -row + (i * row));
    }

    lv_obj_t *hint = lv_label_create(scr);
    lv_label_set_text(hint, "Sel: color\nSw: on/off\nBk: off");
    lv_obj_set_style_text_color(hint, lv_color_hex(0x7AA2B8), 0);
    lv_obj_set_style_text_align(hint, LV_TEXT_ALIGN_CENTER, 0);
    lv_obj_align(hint, LV_ALIGN_BOTTOM_MID, 0, -pad);

    display_unlock();

    s_ui.color = LED_COLOR_RED;
    s_ui.led_on = false;
    ui_refresh_led();
    for (int i = 0; i < BTN_COUNT; i++) {
        ui_refresh_button((button_id_t)i, false);
    }
}

void app_main(void)
{
    ESP_ERROR_CHECK(display_init());
    ESP_ERROR_CHECK(led_init());
    ESP_ERROR_CHECK(buttons_init());

    app_create_ui();
    buttons_set_callback(on_button, NULL);

    /* Wiring check */
    led_set_color(LED_COLOR_RED);
    vTaskDelay(pdMS_TO_TICKS(200));
    led_set_color(LED_COLOR_GREEN);
    vTaskDelay(pdMS_TO_TICKS(200));
    led_set_color(LED_COLOR_BLUE);
    vTaskDelay(pdMS_TO_TICKS(200));
    led_off();
    ui_refresh_led();

    ESP_LOGI(TAG, "ready");
}
