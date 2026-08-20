# Cobox contributor instructions

## Commands

Firmware commands run from the repository root with the `esp` Rust toolchain
(`rust-toolchain.toml`) and ESP-IDF tools installed:

```sh
cargo check
cargo build --release
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --workspace -- -D warnings
cargo run # flashes and opens the ESP32 serial monitor through espflash
```

There is no firmware test harness or individual test command: the binary has
`harness = false`. CI runs the release build, formatting check, and Clippy
command above.

Mobile commands run from `web/`:

```sh
bun install
bunx tsc --noEmit
bun run lint
bun run android # install/update the Android development build
bun run ios
bun run start   # start Metro for the installed development build
bun run web
```

The app uses `react-native-ble-plx`, so test BLE changes in an Expo development
build; Expo Go cannot load it. Rebuild the native app after adding a native
dependency or changing `app.json` plugins/permissions.

## Architecture

- `src/main.rs` owns the ESP32 event loop. It initializes the display, NVS,
  BLE, RGB LED, buttons, and optional Wasm plugin; it drains BLE updates and
  persists each accepted profile before redrawing.
- `src/blob/` owns the blob's generated shape, personality, animation, and
  versioned `BlobConfig` binary representation. `src/storage.rs` stores that
  exact serialized profile in NVS.
- `src/ble.rs` exposes one authenticated BLE GATT service: the settings
  characteristic carries the entire serialized `BlobConfig`; one-byte writes
  are setup rolls. Pairing starts only after the physical Back button opens its
  window, uses MITM bonding with the displayed passkey, and must support
  offset/long reads. The mobile client requests MTU 128 before it reads or
  writes the 106-byte profile.
- `src/display.rs` keeps a heap-backed full-screen text framebuffer for
  discovery, QR, and passkey screens. `Blob` owns separate heap-backed sprite
  buffers. Do not put full display buffers or large render temporaries on the
  ESP32 main-task stack; `sdkconfig.defaults` deliberately gives that task
  16 KiB.
- `web/src/app/index.tsx` is the setup and customization screen.
  `web/src/ble.ts` owns scanning, pairing/read retries, MTU negotiation, and
  base64 GATT transfers. The device profile is authoritative; phone storage
  caches only the paired device ID and re-reads/verifies Cobox before saving
  the cached name.
- `src/plugin.rs` runs a validated Wasm plugin from the raw `plugins`
  partition. `docs/plugins.md` defines the packed ABI and USB installation
  flow; the firmware does not support BLE plugin installation.

## Repository conventions

- Treat `BlobConfig` as a shared wire/storage format. When changing its
  version, length, offsets, validation, or setup fields, update
  `src/blob/config.rs`, the BLE characteristic sizing/handling, and matching
  constants and profile handling in `web/src/ble.ts` together.
- Preserve BLE security boundaries: settings reads/writes require the active
  authenticated connection, partial reads return the requested profile slice,
  and setup rolls are capped on the ESP32 rather than in the app.
- Keep setup rendering separate from blob rendering. Setup/pairing screens use
  `DisplayBundle`'s text framebuffer; after successful pairing, redraw and
  animate the blob while app-driven setup updates arrive.
- Keep plugins constrained to the existing host imports and runtime limits.
  A malformed/trapping plugin is disabled for that boot and must not take down
  the Cobox main loop.
