#include "buttons.h"
#include "chips/board.h"

#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_check.h"
#include "esp_log.h"
#include "driver/gpio.h"

static const char *TAG = "cobox:buttons";

#define BTN_DEBOUNCE_MS  30
#define BTN_POLL_MS      20

static const gpio_num_t s_pins[BTN_COUNT] = {
    [BTN_BACK] = PIN_BTN_BACK,
    [BTN_SWITCH] = PIN_BTN_SWITCH,
    [BTN_SELECT] = PIN_BTN_SELECT,
};

static bool s_stable[BTN_COUNT];
static bool s_raw_last[BTN_COUNT];
static TickType_t s_last_change[BTN_COUNT];
static button_callback_t s_cb;
static void *s_cb_user;

static inline bool read_pressed(button_id_t id)
{
    return gpio_get_level(s_pins[id]) == 0; /* active-low, pull-up */
}

static void buttons_task(void *arg)
{
    (void)arg;
    const TickType_t debounce = pdMS_TO_TICKS(BTN_DEBOUNCE_MS);

    for (int i = 0; i < BTN_COUNT; i++) {
        s_stable[i] = s_raw_last[i] = read_pressed((button_id_t)i);
        s_last_change[i] = xTaskGetTickCount();
    }

    while (1) {
        const TickType_t now = xTaskGetTickCount();

        for (int i = 0; i < BTN_COUNT; i++) {
            const bool raw = read_pressed((button_id_t)i);

            if (raw != s_raw_last[i]) {
                s_raw_last[i] = raw;
                s_last_change[i] = now;
            }

            if ((now - s_last_change[i]) >= debounce && raw != s_stable[i]) {
                s_stable[i] = raw;
                if (s_cb) {
                    button_event_t ev = {
                        .id = (button_id_t)i,
                        .type = raw ? BTN_EVENT_PRESSED : BTN_EVENT_RELEASED,
                    };
                    s_cb(&ev, s_cb_user);
                }
            }
        }

        vTaskDelay(pdMS_TO_TICKS(BTN_POLL_MS));
    }
}

esp_err_t buttons_init(void)
{
    for (int i = 0; i < BTN_COUNT; i++) {
        gpio_reset_pin(s_pins[i]); /* clear prior peripheral routing */
    }

    const gpio_config_t cfg = {
        .pin_bit_mask = (1ULL << PIN_BTN_BACK) | (1ULL << PIN_BTN_SWITCH) | (1ULL << PIN_BTN_SELECT),
        .mode = GPIO_MODE_INPUT,
        .pull_up_en = GPIO_PULLUP_ENABLE,
        .pull_down_en = GPIO_PULLDOWN_DISABLE,
        .intr_type = GPIO_INTR_DISABLE,
    };
    ESP_RETURN_ON_ERROR(gpio_config(&cfg), TAG, "gpio_config failed");

    ESP_RETURN_ON_FALSE(xTaskCreate(buttons_task, "buttons", 3072, NULL, 5, NULL) == pdPASS,
                        ESP_ERR_NO_MEM, TAG, "task create failed");
    return ESP_OK;
}

void buttons_set_callback(button_callback_t cb, void *user_data)
{
    s_cb = cb;
    s_cb_user = user_data;
}

const char *buttons_name(button_id_t id)
{
    switch (id) {
    case BTN_BACK:   return "Back";
    case BTN_SWITCH: return "Switch";
    case BTN_SELECT: return "Select";
    default:         return "?";
    }
}
