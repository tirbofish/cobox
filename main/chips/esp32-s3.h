/**
* ESP32-S3 pin map
* 
* Refer to [docs/pins-esp32_s3.md](docs/pins-esp32_s3.md) for more information.
*/

#pragma once

#include "driver/gpio.h"

#define PIN_LCD_SCLK    GPIO_NUM_12
#define PIN_LCD_MOSI    GPIO_NUM_11
#define PIN_LCD_RST     GPIO_NUM_14
#define PIN_LCD_DC      GPIO_NUM_13
#define PIN_LCD_CS      GPIO_NUM_10
#define PIN_LCD_BL      GPIO_NUM_21

#define PIN_BTN_BACK    GPIO_NUM_4
#define PIN_BTN_SWITCH  GPIO_NUM_5
#define PIN_BTN_SELECT  GPIO_NUM_6

#define PIN_LED_R       GPIO_NUM_1
#define PIN_LED_G       GPIO_NUM_2
#define PIN_LED_B       GPIO_NUM_7
