# cubemaster

Firmware for ESP32-S3-WROOM-1.

Toolchain: Xtensa Rust (`esp` channel, installed via [`espup`](https://github.com/esp-rs/espup)). The required channel is pinned in `rust-toolchain.toml`.

## Prerequisites

- `espup` + the `esp` Rust toolchain (`espup install`, then `. ~/export-esp.sh`)
- `espflash` v4+ (`cargo install espflash --locked`)
- An ESP32-S3 connected via its built-in USB-JTAG-Serial (enumerates as `/dev/ttyACM0` on Linux)

Make sure your shell has the Xtensa toolchain in scope before building:

```bash
. ~/export-esp.sh
```

## Flash and run

```bash
cargo run --release
```

This invokes the runner configured in `.cargo/config.toml`:

```
espflash flash --before usb-reset --after watchdog-reset
```

It flashes the bootloader, partition table, and app, then resets the chip into the application. It does not open a serial monitor.

## View logs (defmt over UART via `esp-println`)

In a **separate terminal**, after flashing:

```bash
espflash monitor \
  --log-format defmt \
  --elf target/xtensa-esp32s3-none-elf/release/cubemaster \
  --non-interactive
```

You should see:

```
[INFO ] Embassy initialized! (cubemaster src/bin/main.rs:55)
[INFO ] Hello world! (cubemaster src/bin/main.rs:71)
[INFO ] Hello world! (cubemaster src/bin/main.rs:71)
...
```

Press `Ctrl+C` to stop the monitor.

### Why `--non-interactive`?

Without `--non-interactive`, `espflash monitor` enters an interactive mode that sets up a terminal raw-mode input reader (for the `Ctrl+R` / `Ctrl+C` shortcuts it prints on startup). On some Linux hosts this fails with:

```
Error:   × I/O error
```

The culprit is usually `ModemManager` probing `/dev/ttyACM*` the moment it appears after reset, racing with espflash's port open. `--non-interactive` skips that input reader entirely and reads the serial stream straight through, which is reliable.

If you prefer a fully interactive monitor (with `Ctrl+R` reset support), the alternative is to tell `ModemManager` to ignore Espressif devices via a udev rule. That's not set up in this repo

## Quick one-liner (flash + monitor in one shot)

```bash
cargo run --release && espflash monitor \
  --log-format defmt \
  --elf target/xtensa-esp32s3-none-elf/release/cubemaster \
  --non-interactive
```

## Project layout

- `src/bin/main.rs` — application entry point (`#[esp_rtos::main]`).
- `.cargo/config.toml` — target, linker flags, and `cargo run` runner.
- `build.rs` — wires in required linker scripts (`linkall.x`, `defmt.x`, etc.) and a friendly linker error handler.
- `rust-toolchain.toml` — pins the `esp` Xtensa toolchain channel.

## Logging stack

- `defmt` — deferred formatting, the log macros (`info!`, `warn!`, ...).
- `esp-println` with the `defmt-espflash` feature — ships defmt frames over UART0.
- `esp-backtrace` — prints a backtrace on panic, also over defmt.

`defmt-rtt` (the more common `defmt` transport on Cortex-M) is intentionally **not** used: it writes to a RAM buffer that a JTAG debugger reads, which requires a working `probe-rs` flow. Going through UART with `esp-println` means `espflash monitor` (which talks to the same USB-CDC serial port espflash flashes over) is all you need — no separate debugger, no RTT control block scanning.

## Troubleshooting

- **`[WARN ] Monitor options were provided, but --monitor/-M flag isn't set`**: harmless cosmetic warning from espflash 4.x — it treats `--before` / `--after` as "monitor options" even though we're not monitoring. Ignore it.
- **`cargo run` fails to connect to `/dev/ttyACM0`**: unplug and replug the ESP32-S3, wait a couple of seconds for `ModemManager` to finish probing, then retry. If it's persistent, check `fuser /dev/ttyACM0` to see what's holding it (VSCode serial monitor, `screen`, `minicom`, another `espflash monitor`, ...).
- **Garbled output in the monitor**: confirm you passed `--log-format defmt` and `--elf <path-to-elf>`. The ELF is needed to decode defmt interning.
- **Chip stuck in download mode after flashing** (`boot:0x0 (DOWNLOAD(USB/UART0))`): the `--before usb-reset --after watchdog-reset` flags in the runner handle this for the S3's USB-JTAG-Serial. If you flash with different flags and hit this, add them back.
- **Build error "`panic-probe` only supports Cortex-M targets"**: make sure you're on the fixed `Cargo.toml` (uses `esp-backtrace`, not `panic-probe`).
