#include <stdint.h>

#include "esp_log.h"

#include "display.h"
#include "gen/blob_gen.h"
#include "led.h"
#include "lvgl.h"

static const char *TAG = "cobox";

void app_main(void)
{
    ESP_ERROR_CHECK(display_init());
    ESP_ERROR_CHECK(led_init());
    generate_blob();

    ESP_LOGI(TAG, "boot ready");
}
