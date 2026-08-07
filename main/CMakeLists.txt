idf_component_register(SRCS "main.c" "display.c" "led.c" "buttons.c"
                            "displays/st7789.c" "displays/st7735.c"
                    INCLUDE_DIRS "."
                    REQUIRES driver esp_lcd esp_lvgl_port esp_driver_ledc esp_driver_gpio)

# Optional: force ST7735 without editing display.h
# target_compile_definitions(${COMPONENT_LIB} PRIVATE COBOX_USE_ST7735)
