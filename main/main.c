#include <stdio.h>

#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "display.h"
#include "buttons.h"
#include "led.h"
#include "lvgl.h"

static const char *TAG = "cobox";

typedef struct {
    lv_obj_t *title;
    lv_obj_t *led_label;
    lv_obj_t *raw_label;
    lv_obj_t *hint;
    lv_obj_t *btn_labels[BTN_COUNT];
    led_color_t color;
    bool led_on;
} ui_t;

static ui_t s_ui;

static void ui_refresh_raw(void)
{
    char buf[64];
    snprintf(buf, sizeof(buf), "raw B%d S%d Sel%d",
             buttons_gpio_level(BTN_BACK),
             buttons_gpio_level(BTN_SWITCH),
             buttons_gpio_level(BTN_SELECT));

    if (display_lock(50)) {
        lv_label_set_text(s_ui.raw_label, buf);
        display_unlock();
    }
}

static void raw_monitor_task(void *arg)
{
    (void)arg;
    while (1) {
        ui_refresh_raw();
        vTaskDelay(pdMS_TO_TICKS(100));
    }
}

static void ui_refresh_led(void)
{
    char buf[48];
    if (s_ui.led_on) {
        snprintf(buf, sizeof(buf), "LED: %s", led_color_name(s_ui.color));
        led_set_color(s_ui.color);
    } else {
        snprintf(buf, sizeof(buf), "LED: Off");
        led_off();
    }

    if (display_lock(50)) {
        lv_label_set_text(s_ui.led_label, buf);
        display_unlock();
    }
}

static void ui_refresh_button(button_id_t id, bool pressed)
{
    char buf[32];
    snprintf(buf, sizeof(buf), "%s: %s", buttons_name(id), pressed ? "DOWN" : "up");

    if (display_lock(50)) {
        lv_label_set_text(s_ui.btn_labels[id], buf);
        display_unlock();
    }
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
        /* Cycle colors while LED is considered on */
        s_ui.color = (led_color_t)((s_ui.color + 1) % LED_COLOR_COUNT);
        if (s_ui.color == LED_COLOR_OFF) {
            s_ui.color = LED_COLOR_RED;
        }
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
        /* More obvious than only turning LED off */
        s_ui.led_on = false;
        s_ui.color = LED_COLOR_OFF;
        led_set_rgb(40, 40, 40); /* brief dim white flash */
        vTaskDelay(pdMS_TO_TICKS(80));
        ui_refresh_led();
        break;

    default:
        break;
    }
}

static void app_create_ui(void)
{
    display_lock(0);

    lv_obj_t *scr = lv_screen_active();
    lv_obj_set_style_bg_color(scr, lv_color_hex(0x101820), 0);

    s_ui.title = lv_label_create(scr);
    lv_label_set_text(s_ui.title, "cobox test");
    lv_obj_set_style_text_color(s_ui.title, lv_color_hex(0xE8F1F8), 0);
    lv_obj_align(s_ui.title, LV_ALIGN_TOP_MID, 0, 12);

    s_ui.led_label = lv_label_create(scr);
    lv_obj_set_style_text_color(s_ui.led_label, lv_color_hex(0x7CFFB2), 0);
    lv_obj_align(s_ui.led_label, LV_ALIGN_TOP_MID, 0, 40);

    s_ui.raw_label = lv_label_create(scr);
    lv_obj_set_style_text_color(s_ui.raw_label, lv_color_hex(0x8899AA), 0);
    lv_obj_align(s_ui.raw_label, LV_ALIGN_TOP_MID, 0, 62);

    for (int i = 0; i < BTN_COUNT; i++) {
        s_ui.btn_labels[i] = lv_label_create(scr);
        lv_obj_set_style_text_color(s_ui.btn_labels[i], lv_color_hex(0xC8D6E0), 0);
        lv_obj_align(s_ui.btn_labels[i], LV_ALIGN_LEFT_MID, 24, -20 + (i * 28));
    }

    s_ui.hint = lv_label_create(scr);
    lv_label_set_text(s_ui.hint,
                      "Select: color\n"
                      "Switch: on/off\n"
                      "Back: LED off");
    lv_obj_set_style_text_color(s_ui.hint, lv_color_hex(0x7AA2B8), 0);
    lv_obj_set_style_text_align(s_ui.hint, LV_TEXT_ALIGN_CENTER, 0);
    lv_obj_align(s_ui.hint, LV_ALIGN_BOTTOM_MID, 0, -16);

    display_unlock();

    s_ui.color = LED_COLOR_OFF;
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
    xTaskCreate(raw_monitor_task, "btn_raw", 2048, NULL, 3, NULL);

    /* Startup LED blink so wiring is obvious */
    led_set_color(LED_COLOR_RED);
    vTaskDelay(pdMS_TO_TICKS(200));
    led_set_color(LED_COLOR_GREEN);
    vTaskDelay(pdMS_TO_TICKS(200));
    led_set_color(LED_COLOR_BLUE);
    vTaskDelay(pdMS_TO_TICKS(200));
    led_off();
    ui_refresh_led();

    ESP_LOGI(TAG, "UI test running");
}
