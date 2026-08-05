#include "buttons.h"

#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_check.h"
#include "esp_log.h"
#include "driver/gpio.h"

static const char *TAG = "cobox:buttons";

#define PIN_BTN_BACK     GPIO_NUM_19
#define PIN_BTN_SWITCH   GPIO_NUM_26
#define PIN_BTN_SELECT   GPIO_NUM_25

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

static bool read_raw_pressed(button_id_t id)
{
    /* Active-low with internal pull-up: pressed == 0 */
    return gpio_get_level(s_pins[id]) == 0;
}

static void buttons_task(void *arg)
{
    (void)arg;
    const TickType_t debounce_ticks = pdMS_TO_TICKS(BTN_DEBOUNCE_MS);

    for (int i = 0; i < BTN_COUNT; i++) {
        s_stable[i] = read_raw_pressed((button_id_t)i);
        s_raw_last[i] = s_stable[i];
        s_last_change[i] = xTaskGetTickCount();
    }

    while (1) {
        const TickType_t now = xTaskGetTickCount();

        for (int i = 0; i < BTN_COUNT; i++) {
            const bool raw = read_raw_pressed((button_id_t)i);

            if (raw != s_raw_last[i]) {
                s_raw_last[i] = raw;
                s_last_change[i] = now;
            }

            if ((now - s_last_change[i]) >= debounce_ticks && raw != s_stable[i]) {
                s_stable[i] = raw;

                button_event_t ev = {
                    .id = (button_id_t)i,
                    .type = raw ? BTN_EVENT_PRESSED : BTN_EVENT_RELEASED,
                };

                ESP_LOGI(TAG, "%s %s (gpio%d level=%d)",
                         buttons_name(ev.id),
                         ev.type == BTN_EVENT_PRESSED ? "pressed" : "released",
                         (int)s_pins[i],
                         gpio_get_level(s_pins[i]));

                if (s_cb) {
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
        const gpio_num_t pin = s_pins[i];

        /* Clear any prior peripheral routing (I2C, etc.) */
        ESP_RETURN_ON_ERROR(gpio_reset_pin(pin), TAG, "reset_pin failed");
        ESP_RETURN_ON_ERROR(gpio_set_direction(pin, GPIO_MODE_INPUT), TAG, "set_direction failed");
        ESP_RETURN_ON_ERROR(gpio_set_pull_mode(pin, GPIO_PULLUP_ONLY), TAG, "pull_mode failed");
        ESP_RETURN_ON_ERROR(gpio_pullup_en(pin), TAG, "pullup_en failed");

        ESP_LOGI(TAG, "%s on GPIO%d idle_level=%d (expect 1)",
                 buttons_name((button_id_t)i), (int)pin, gpio_get_level(pin));
    }

    BaseType_t ok = xTaskCreate(buttons_task, "buttons", 3072, NULL, 5, NULL);
    ESP_RETURN_ON_FALSE(ok == pdPASS, ESP_ERR_NO_MEM, TAG, "task create failed");

    ESP_LOGI(TAG, "Buttons ready (back=%d switch=%d select=%d)",
             PIN_BTN_BACK, PIN_BTN_SWITCH, PIN_BTN_SELECT);
    return ESP_OK;
}

void buttons_set_callback(button_callback_t cb, void *user_data)
{
    s_cb = cb;
    s_cb_user = user_data;
}

bool buttons_is_pressed(button_id_t id)
{
    if (id >= BTN_COUNT) {
        return false;
    }
    return s_stable[id];
}

int buttons_gpio_level(button_id_t id)
{
    if (id >= BTN_COUNT) {
        return -1;
    }
    return gpio_get_level(s_pins[id]);
}

gpio_num_t buttons_gpio_num(button_id_t id)
{
    if (id >= BTN_COUNT) {
        return GPIO_NUM_NC;
    }
    return s_pins[id];
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
