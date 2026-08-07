/**
* Classic ESP32 pin map
*/

#pragma once

#include "driver/gpio.h"

#define PIN_LCD_SCLK    GPIO_NUM_18
#define PIN_LCD_MOSI    GPIO_NUM_23
#define PIN_LCD_RST     GPIO_NUM_15
#define PIN_LCD_DC      GPIO_NUM_16
#define PIN_LCD_CS      GPIO_NUM_5
#define PIN_LCD_BL      GPIO_NUM_4

#define PIN_BTN_BACK    GPIO_NUM_19
#define PIN_BTN_SWITCH  GPIO_NUM_26
#define PIN_BTN_SELECT  GPIO_NUM_25

#define PIN_LED_R       GPIO_NUM_27
#define PIN_LED_G       GPIO_NUM_32
#define PIN_LED_B       GPIO_NUM_33
