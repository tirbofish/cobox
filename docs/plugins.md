# Cobox plugins

Plugins are one WebAssembly binary in the raw `plugins` partition. There is no
BLE installation path yet: build a `.wasm`, pack it over USB, then flash it.

## ABI

The partition begins with this 16-byte little-endian header:

| Bytes | Field |
| --- | --- |
| 0..4 | ASCII magic `CBXW` |
| 4..6 | ABI version `1` (`u16`) |
| 6..8 | reserved, zero (`u16`) |
| 8..12 | Wasm payload length (`u32`) |
| 12..16 | IEEE CRC32 of the payload (`u32`) |

The runtime accepts payloads through 128 KiB. Modules must export:

```text
cobox_init() -> i32
cobox_tick(now_ms: i32) -> i32
```

Both return zero on success. A nonzero return or trap disables the plugin for
the current boot. `now_ms` is monotonic milliseconds since boot, saturated at
`i32::MAX`; ticks run every 250 ms with 20,000 Wasmi fuel units each.

Only these imports are available, from module `cobox`:

```text
cobox_set_led(r: i32, g: i32, b: i32)
cobox_set_expression(bob_offset: i32, eye_scale: i32)
```

LED values clamp to `0..=255`; bob offset to `-20..=20`; and eye scale to
`25..=200` percent. Commands are queued by Wasm and applied on Cobox's main
loop. An expression must be sent again on each tick to remain active.

No WASI, filesystem, networking, ESP-IDF imports, pointers, display, or SPI
access are exposed. Modules may define at most one linear memory, explicitly
bounded to one 64 KiB page.

## USB installation

Pack a Wasm module into the complete `plugins` partition image:

```sh
python3 tools/pack_plugin.py plugin.wasm plugins.bin
espflash write-bin --chip esp32 --port /dev/ttyUSB0 0x1de000 plugins.bin
```

Replace `/dev/ttyUSB0` with the Cobox serial port. `0x1de000` is the start of
the raw partition; the packer writes its exact `0x22000`-byte size. Flashing a
new Cobox app must use `partitions.csv`, which keeps `factory` at 1848 KiB and
does not overwrite this partition. A future authenticated BLE uploader can
write through the `PluginStore` boundary; this firmware runtime only reads it.
