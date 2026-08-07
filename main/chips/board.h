#pragma once

#if CONFIG_IDF_TARGET_ESP32S3
#include "chips/esp32-s3.h"
#elif CONFIG_IDF_TARGET_ESP32
#include "chips/esp32.h"
#else
#error "Unsupported IDF target — add a chips/<target>.h pin map"
#endif
