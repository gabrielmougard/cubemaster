# shared — CubeMaster Protocol & Types

`no_std`-compatible crate with no target-specific or platform
dependencies. Shared between `firmware/` and `companion/`.

## Module Layout

```
shared/
  src/
    lib.rs          — crate root, #![no_std], re-exports
    ble.rs           — GATT service/characteristic UUIDs
    led.rs           — LED geometry constants, color types
    protocol.rs      — wire format (message type enum, coders)
    sound.rs         — audio format constants (sample rate, bit depth, ...)
```

## Guidelines

- **No platform deps**: no `std`, no `alloc` unless gated. Firmware
  uses `#![no_std]` and must be able to depend on this crate without
  pulling in an allocator.
- **Serde**: use `serde` with `default-features = false` + `derive`.
  For JSON in `no_std` contexts, use `serde-json-core`.
- **Types only**: no I/O, no filesystem, no networking. Pure data
  structures and constants.
- **Extend for protocol**: when adding a new BLE message type, add
  the variant to the message enum here and implement its encode/decode.
  Firmware and companion app will both use the same types.

## Future Modules (add as needed)

- `intent.rs` — intent + entity type definitions (for T-008b)
- `pattern.rs` — `.cubepattern` binary format header struct
- `config.rs` — keyword configuration, cube settings
- `error.rs` — shared error types

## Versioning

This crate follows the workspace version. Breaking changes to the
wire protocol should bump the minor version and be documented in the
spec (`doc/cubemaster-mvp.bs`).
