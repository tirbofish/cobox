# Pins - ESP32

Target: `esp32` (original).

## Display (SPI2)

| Signal | GPIO | Notes |
|--------|------|--------|
| SCLK   | 18   | SCL |
| MOSI   | 23   | SDA |
| RST    | 15   | |
| DC     | 16   | |
| CS     | 5 / NC | 5 for ST7735; NC for ST7789 |
| BL     | 4    | backlight, active high |

## Buttons (active-low, pull-up)

| Button | GPIO |
|--------|------|
| Back   | 19   |
| Switch | 26   |
| Select | 25   |

## RGB LED

| Channel | GPIO |
|---------|------|
| R       | 27   |
| G       | 32   |
| B       | 33   |
