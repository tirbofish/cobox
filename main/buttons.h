#pragma once

#include <stdbool.h>
#include "esp_err.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    BTN_BACK = 0,
    BTN_SWITCH,
    BTN_SELECT,
    BTN_COUNT,
} button_id_t;

typedef enum {
    BTN_EVENT_PRESSED = 0,
    BTN_EVENT_RELEASED,
} button_event_type_t;

typedef struct {
    button_id_t id;
    button_event_type_t type;
} button_event_t;

typedef void (*button_callback_t)(const button_event_t *event, void *user_data);

esp_err_t buttons_init(void);
void buttons_set_callback(button_callback_t cb, void *user_data);
const char *buttons_name(button_id_t id);

#ifdef __cplusplus
}
#endif
