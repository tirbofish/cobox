idf_component_register(SRCS "main.c" "display.c" "led.c" "buttons.c"
                    INCLUDE_DIRS "."
                    REQUIRES driver esp_lcd esp_lvgl_port esp_driver_ledc esp_driver_gpio)
