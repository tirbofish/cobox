#!/usr/bin/env python3
"""Pack one Cobox WebAssembly module for the raw plugins partition."""

import argparse
import struct
import zlib
from pathlib import Path

ABI_VERSION = 1
MAX_MODULE_BYTES = 128 * 1024
PARTITION_BYTES = 0x22000
HEADER = struct.Struct("<4sHHII")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("wasm", type=Path, help="input plugin.wasm")
    parser.add_argument("image", type=Path, help="output plugins.bin")
    args = parser.parse_args()

    wasm = args.wasm.read_bytes()
    if len(wasm) > MAX_MODULE_BYTES:
        parser.error(f"module is {len(wasm)} bytes; limit is {MAX_MODULE_BYTES}")
    if len(wasm) < 8 or wasm[:4] != b"\0asm":
        parser.error("input is not a WebAssembly binary")

    header = HEADER.pack(b"CBXW", ABI_VERSION, 0, len(wasm), zlib.crc32(wasm) & 0xFFFFFFFF)
    image = header + wasm
    if len(image) > PARTITION_BYTES:
        parser.error("module does not fit the plugins partition")
    image += b"\xff" * (PARTITION_BYTES - len(image))
    assert len(image) == PARTITION_BYTES
    assert HEADER.unpack(image[: HEADER.size])[4] == zlib.crc32(wasm) & 0xFFFFFFFF
    args.image.write_bytes(image)
    print(f"packed {len(wasm)} bytes into {args.image} ({len(image)} bytes)")


if __name__ == "__main__":
    main()
