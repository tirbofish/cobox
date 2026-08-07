# Pins — ESP32-S3

Target: `esp32s3`. Chosen from Espressif GPIO rules
([IDF GPIO notes](https://docs.espressif.com/projects/esp-idf/en/stable/esp32s3/api-reference/peripherals/gpio.html)):
avoid strapping (0/3/45/46), USB-JTAG (19/20), flash/PSRAM (26–32),
octal SPI (33–37 on R8 modules), UART0 (43/44).

## Display (SPI2 / FSPI)

| Signal | GPIO | Notes |
|--------|------|--------|
| SCLK   | 12   | FSPICLK |
| MOSI   | 11   | FSPID / SDA |
| RST    | 14   | |
| DC     | 13   | |
| CS     | 10   | ST7735; ST7789 still uses NC in firmware |
| BL     | 21   | backlight, active high |

## Buttons (active-low, pull-up)

| Button | GPIO |
|--------|------|
| Back   | 4    |
| Switch | 5    |
| Select | 6    |

## RGB LED

| Channel | GPIO |
|---------|------|
| R       | 1    |
| G       | 2    |
| B       | 7    |
