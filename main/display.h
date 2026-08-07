#pragma once

#include <stdbool.h>
#include <stdint.h>
#include "esp_err.h"

#ifdef __cplusplus
extern "C" {
#endif

#define COBOX_USE_ST7735

#if defined(COBOX_USE_ST7735)
#include "displays/st7735.h"
#define LCD_H_RES ST7735_H_RES
#define LCD_V_RES ST7735_V_RES
#else
#include "displays/st7789.h"
#define LCD_H_RES ST7789_H_RES
#define LCD_V_RES ST7789_V_RES
#endif

esp_err_t display_init(void);
bool display_lock(uint32_t timeout_ms);
void display_unlock(void);

#ifdef __cplusplus
}
#endif
