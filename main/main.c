/*
 * cobox — ST7789 SPI LCD + LVGL via esp_lvgl_port
 *
 * Wiring (module pin → ESP32):
 *   GND - Ground
 *   VCC - 3.3V
 *   SCL - GPIO18
 *   SDA - GPIO23
 *   RST - GPIO15
 *   DC  - GPIO2
 *   BLK - GPIO4
 *
 * CS is not wired (tied low on many modules) → GPIO_NUM_NC
 */

#include <stdio.h>
#include <string.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_err.h"
#include "esp_log.h"
#include "esp_check.h"
#include "esp_heap_caps.h"
#include "driver/gpio.h"
#include "driver/spi_master.h"
#include "esp_lcd_panel_io.h"
#include "esp_lcd_panel_vendor.h"
#include "esp_lcd_panel_ops.h"
#include "esp_lcd_panel_st7789.h"
#include "esp_lvgl_port.h"
#include "lvgl.h"

static const char *TAG = "cobox";

/* Matches working Rust/mipidsi setup: 240x240 ST7789 */
#define LCD_H_RES               240
#define LCD_V_RES               240

#define LCD_SPI_HOST            SPI2_HOST
#define LCD_PIXEL_CLOCK_HZ      (26 * 1000 * 1000)
#define LCD_CMD_BITS            8
#define LCD_PARAM_BITS          8
#define LCD_BITS_PER_PIXEL      16
#define LCD_DRAW_BUFF_HEIGHT    40

/* Rust used backlight.set_high() */
#define LCD_BL_ON_LEVEL         1

/* Pins from your wiring */
#define PIN_LCD_SCLK            GPIO_NUM_18
#define PIN_LCD_MOSI            GPIO_NUM_23
#define PIN_LCD_RST             GPIO_NUM_15
#define PIN_LCD_DC              GPIO_NUM_2
#define PIN_LCD_CS              GPIO_NUM_NC
#define PIN_LCD_BL              GPIO_NUM_4

static esp_lcd_panel_io_handle_t lcd_io = NULL;
static esp_lcd_panel_handle_t lcd_panel = NULL;
static lv_display_t *lvgl_disp = NULL;

static void backlight_on(void)
{
    gpio_set_level(PIN_LCD_BL, LCD_BL_ON_LEVEL);
}

static void backlight_init(void)
{
    const gpio_config_t bk_gpio_config = {
        .mode = GPIO_MODE_OUTPUT,
        .pin_bit_mask = 1ULL << PIN_LCD_BL,
    };
    ESP_ERROR_CHECK(gpio_config(&bk_gpio_config));
    /* On immediately so a dark panel isn't mistaken for a driver failure */
    backlight_on();
}

/* Solid fill via esp_lcd — proves SPI + panel before LVGL */
static void lcd_fill_color(uint16_t rgb565)
{
    const size_t line_pixels = LCD_H_RES;
    uint16_t *line = heap_caps_malloc(line_pixels * sizeof(uint16_t), MALLOC_CAP_DMA);
    if (!line) {
        ESP_LOGE(TAG, "fill alloc failed");
        return;
    }

    /* SPI panels expect big-endian RGB565 */
    const uint16_t be = (uint16_t)((rgb565 << 8) | (rgb565 >> 8));
    for (size_t i = 0; i < line_pixels; i++) {
        line[i] = be;
    }

    for (int y = 0; y < LCD_V_RES; y++) {
        ESP_ERROR_CHECK(esp_lcd_panel_draw_bitmap(lcd_panel, 0, y, LCD_H_RES, y + 1, line));
    }

    heap_caps_free(line);
}

static esp_err_t app_lcd_init(void)
{
    backlight_init();

    ESP_LOGI(TAG, "Init SPI bus");
    const spi_bus_config_t buscfg = {
        .sclk_io_num = PIN_LCD_SCLK,
        .mosi_io_num = PIN_LCD_MOSI,
        .miso_io_num = GPIO_NUM_NC,
        .quadwp_io_num = GPIO_NUM_NC,
        .quadhd_io_num = GPIO_NUM_NC,
        .max_transfer_sz = LCD_H_RES * LCD_DRAW_BUFF_HEIGHT * sizeof(uint16_t),
    };
    ESP_RETURN_ON_ERROR(spi_bus_initialize(LCD_SPI_HOST, &buscfg, SPI_DMA_CH_AUTO), TAG, "SPI init failed");

    ESP_LOGI(TAG, "Install panel IO");
    const esp_lcd_panel_io_spi_config_t io_config = {
        .dc_gpio_num = PIN_LCD_DC,
        .cs_gpio_num = PIN_LCD_CS,
        .pclk_hz = LCD_PIXEL_CLOCK_HZ,
        .lcd_cmd_bits = LCD_CMD_BITS,
        .lcd_param_bits = LCD_PARAM_BITS,
        /* Rust/mipidsi required MODE_3 for this panel */
        .spi_mode = 3,
        .trans_queue_depth = 10,
    };
    ESP_RETURN_ON_ERROR(esp_lcd_new_panel_io_spi((esp_lcd_spi_bus_handle_t)LCD_SPI_HOST, &io_config, &lcd_io),
                        TAG, "panel IO failed");

    ESP_LOGI(TAG, "Install ST7789 driver");
    const esp_lcd_panel_dev_config_t panel_config = {
        .reset_gpio_num = PIN_LCD_RST,
        .rgb_ele_order = LCD_RGB_ELEMENT_ORDER_RGB,
        .bits_per_pixel = LCD_BITS_PER_PIXEL,
    };
    ESP_RETURN_ON_ERROR(esp_lcd_new_panel_st7789(lcd_io, &panel_config, &lcd_panel), TAG, "ST7789 failed");

    ESP_ERROR_CHECK(esp_lcd_panel_reset(lcd_panel));
    ESP_ERROR_CHECK(esp_lcd_panel_init(lcd_panel));
    ESP_ERROR_CHECK(esp_lcd_panel_invert_color(lcd_panel, true));
    ESP_ERROR_CHECK(esp_lcd_panel_mirror(lcd_panel, false, false));
    ESP_ERROR_CHECK(esp_lcd_panel_disp_on_off(lcd_panel, true));

    backlight_on();

    /* Bright red = hardware path works even if LVGL config is wrong */
    ESP_LOGI(TAG, "Painting solid red test pattern");
    lcd_fill_color(0xF800); /* RGB565 red */
    vTaskDelay(pdMS_TO_TICKS(800));

    ESP_LOGI(TAG, "LCD ready");
    return ESP_OK;
}

static esp_err_t app_lvgl_init(void)
{
    const lvgl_port_cfg_t lvgl_cfg = ESP_LVGL_PORT_INIT_CONFIG();
    ESP_RETURN_ON_ERROR(lvgl_port_init(&lvgl_cfg), TAG, "lvgl_port_init failed");

    const lvgl_port_display_cfg_t disp_cfg = {
        .io_handle = lcd_io,
        .panel_handle = lcd_panel,
        .buffer_size = LCD_H_RES * LCD_DRAW_BUFF_HEIGHT,
        .double_buffer = true,
        .hres = LCD_H_RES,
        .vres = LCD_V_RES,
        .monochrome = false,
        .color_format = LV_COLOR_FORMAT_RGB565,
        .rotation = {
            .swap_xy = false,
            .mirror_x = false,
            .mirror_y = false,
        },
        .flags = {
            .buff_dma = true,
            .swap_bytes = true,
        },
    };
    lvgl_disp = lvgl_port_add_disp(&disp_cfg);
    ESP_RETURN_ON_FALSE(lvgl_disp, ESP_FAIL, TAG, "lvgl_port_add_disp failed");

    ESP_LOGI(TAG, "LVGL ready");
    return ESP_OK;
}

static void app_create_ui(void)
{
    lvgl_port_lock(0);

    lv_obj_t *scr = lv_screen_active();
    lv_obj_set_style_bg_color(scr, lv_color_hex(0xFFFFFF), 0);

    lv_obj_t *title = lv_label_create(scr);
    lv_label_set_text(title, "cobox");
    lv_obj_set_style_text_color(title, lv_color_hex(0x000000), 0);
    lv_obj_align(title, LV_ALIGN_CENTER, 0, -16);

    lv_obj_t *sub = lv_label_create(scr);
    lv_label_set_text(sub, "LVGL + ST7789");
    lv_obj_set_style_text_color(sub, lv_color_hex(0x333333), 0);
    lv_obj_align(sub, LV_ALIGN_CENTER, 0, 20);

    lvgl_port_unlock();
}

void app_main(void)
{
    ESP_ERROR_CHECK(app_lcd_init());
    ESP_ERROR_CHECK(app_lvgl_init());
    app_create_ui();

    ESP_LOGI(TAG, "UI running (BL on-level=%d)", LCD_BL_ON_LEVEL);
}
