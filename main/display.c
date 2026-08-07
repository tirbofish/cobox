#include "display.h"
#include "chips/board.h"

#include "esp_check.h"
#include "esp_log.h"
#include "esp_heap_caps.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/semphr.h"
#include "driver/gpio.h"
#include "driver/spi_master.h"
#include "esp_lcd_panel_io.h"
#include "esp_lvgl_port.h"

static const char *TAG = "cobox:display";

#define LCD_SPI_HOST         SPI2_HOST
/* ~1/10 screen — dual buf overlaps render with DMA (LVGL ST7735 example) */
#define LCD_DRAW_BUFF_LINES  (LCD_V_RES / 10)
#if defined(COBOX_USE_ST7735)
#define LCD_CS               PIN_LCD_CS
#define LCD_SPI_MODE         ST7735_SPI_MODE
#define LCD_PCLK_HZ          ST7735_PCLK_HZ
#else
#define LCD_CS               GPIO_NUM_NC
#define LCD_SPI_MODE         ST7789_SPI_MODE
#define LCD_PCLK_HZ          ST7789_PCLK_HZ
#endif

static esp_lcd_panel_io_handle_t lcd_io;
static lv_display_t *s_disp;
static SemaphoreHandle_t s_color_done;
static volatile bool s_color_busy;

static bool on_color_trans_done(esp_lcd_panel_io_handle_t io, esp_lcd_panel_io_event_data_t *edata, void *user_ctx)
{
    (void)io;
    (void)edata;
    BaseType_t hp = pdFALSE;
    s_color_busy = false;
    xSemaphoreGiveFromISR(s_color_done, &hp);
    lv_display_flush_ready((lv_display_t *)user_ctx);
    return hp == pdTRUE;
}

static void lcd_wait_idle(void)
{
    while (s_color_busy) {
        /* Wake from DMA ISR — no tick latency, no CPU spin / WDT */
        if (xSemaphoreTake(s_color_done, pdMS_TO_TICKS(100)) != pdTRUE) {
            ESP_LOGW(TAG, "SPI color timeout — resetting busy flag");
            s_color_busy = false;
            break;
        }
    }
}

static void lcd_send_cmd(lv_display_t *disp, const uint8_t *cmd, size_t cmd_size,
                         const uint8_t *param, size_t param_size)
{
    (void)disp;
    (void)cmd_size;
    lcd_wait_idle();
    ESP_ERROR_CHECK(esp_lcd_panel_io_tx_param(lcd_io, cmd[0], param, param_size));
}

static void lcd_send_color(lv_display_t *disp, const uint8_t *cmd, size_t cmd_size,
                           uint8_t *param, size_t param_size)
{
    (void)disp;
    (void)cmd_size;
    lcd_wait_idle();
    while (xSemaphoreTake(s_color_done, 0) == pdTRUE) {
        /* drain stale gives */
    }
    lv_draw_sw_rgb565_swap(param, param_size / 2);
    s_color_busy = true;
    ESP_ERROR_CHECK(esp_lcd_panel_io_tx_color(lcd_io, cmd[0], param, param_size));
}

static esp_err_t lcd_hw_reset(void)
{
    const gpio_config_t rst = {
        .mode = GPIO_MODE_OUTPUT,
        .pin_bit_mask = 1ULL << PIN_LCD_RST,
    };
    ESP_RETURN_ON_ERROR(gpio_config(&rst), TAG, "RST gpio failed");
    gpio_set_level(PIN_LCD_RST, 0);
    vTaskDelay(pdMS_TO_TICKS(20));
    gpio_set_level(PIN_LCD_RST, 1);
    vTaskDelay(pdMS_TO_TICKS(120));
    return ESP_OK;
}

esp_err_t display_init(void)
{
    s_color_done = xSemaphoreCreateBinary();
    ESP_RETURN_ON_FALSE(s_color_done, ESP_ERR_NO_MEM, TAG, "color sem failed");

    const gpio_config_t bk = {
        .mode = GPIO_MODE_OUTPUT,
        .pin_bit_mask = 1ULL << PIN_LCD_BL,
    };
    ESP_RETURN_ON_ERROR(gpio_config(&bk), TAG, "BL gpio failed");
    gpio_set_level(PIN_LCD_BL, 1);

    ESP_RETURN_ON_ERROR(lcd_hw_reset(), TAG, "reset failed");

    const spi_bus_config_t buscfg = {
        .sclk_io_num = PIN_LCD_SCLK,
        .mosi_io_num = PIN_LCD_MOSI,
        .miso_io_num = GPIO_NUM_NC,
        .quadwp_io_num = GPIO_NUM_NC,
        .quadhd_io_num = GPIO_NUM_NC,
        .max_transfer_sz = LCD_H_RES * LCD_DRAW_BUFF_LINES * sizeof(uint16_t),
    };
    ESP_RETURN_ON_ERROR(spi_bus_initialize(LCD_SPI_HOST, &buscfg, SPI_DMA_CH_AUTO), TAG, "SPI init failed");

    const esp_lcd_panel_io_spi_config_t io_config = {
        .dc_gpio_num = PIN_LCD_DC,
        .cs_gpio_num = LCD_CS,
        .pclk_hz = LCD_PCLK_HZ,
        .lcd_cmd_bits = 8,
        .lcd_param_bits = 8,
        .spi_mode = LCD_SPI_MODE,
        .trans_queue_depth = 10,
    };
    ESP_RETURN_ON_ERROR(esp_lcd_new_panel_io_spi((esp_lcd_spi_bus_handle_t)LCD_SPI_HOST, &io_config, &lcd_io),
                        TAG, "panel IO failed");

    const lvgl_port_cfg_t lvgl_cfg = ESP_LVGL_PORT_INIT_CONFIG();
    ESP_RETURN_ON_ERROR(lvgl_port_init(&lvgl_cfg), TAG, "lvgl_port_init failed");

    ESP_RETURN_ON_FALSE(display_lock(0), ESP_FAIL, TAG, "lvgl lock failed");
#if defined(COBOX_USE_ST7735)
    s_disp = st7735_create(lcd_send_cmd, lcd_send_color);
#else
    s_disp = st7789_create(lcd_send_cmd, lcd_send_color);
#endif
    display_unlock();
    ESP_RETURN_ON_FALSE(s_disp, ESP_FAIL, TAG, "display create failed");

    const esp_lcd_panel_io_callbacks_t cbs = {
        .on_color_trans_done = on_color_trans_done,
    };
    ESP_RETURN_ON_ERROR(esp_lcd_panel_io_register_event_callbacks(lcd_io, &cbs, s_disp), TAG, "io cb failed");

    const size_t buf_size = LCD_H_RES * LCD_DRAW_BUFF_LINES * sizeof(uint16_t);
    void *buf1 = heap_caps_malloc(buf_size, MALLOC_CAP_DMA | MALLOC_CAP_INTERNAL);
    void *buf2 = heap_caps_malloc(buf_size, MALLOC_CAP_DMA | MALLOC_CAP_INTERNAL);
    ESP_RETURN_ON_FALSE(buf1 && buf2, ESP_ERR_NO_MEM, TAG, "draw buffer alloc failed");

    lv_display_set_color_format(s_disp, LV_COLOR_FORMAT_RGB565);
    lv_display_set_buffers(s_disp, buf1, buf2, buf_size, LV_DISPLAY_RENDER_MODE_PARTIAL);
    return ESP_OK;
}

bool display_lock(uint32_t timeout_ms)
{
    return lvgl_port_lock(timeout_ms);
}

void display_unlock(void)
{
    lvgl_port_unlock();
}
