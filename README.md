# cubemaster

Intelligent role-play companion cube — ESP32-S3 firmware + Dioxus desktop app.

## Workspace layout

```
cubemaster/               # Cargo workspace root
├── firmware/             # ESP32-S3 embedded firmware (Embassy RTOS)
├── companion/            # Dioxus desktop app (sound banks, cube control)
├── shared/               # Protocol types (no_std, shared by firmware & companion)
├── doc/                  # Bikeshed engineering spec + PlantUML diagrams
├── tasks/                # Epics and tickets
└── Taskfile.yml          # doc build tasks
```

Toolchain: Xtensa Rust (`esp` channel, installed via [espup](https://github.com/esp-rs/espup)). The required channel is pinned in `rust-toolchain.toml`.

## Prerequisites

- `espup` + the `esp` Rust toolchain (`espup install`, then `. ~/export-esp.sh`)
- `espflash` v4+ (`cargo install espflash --locked`)
- An ESP32-S3 connected via its built-in USB-JTAG-Serial (enumerates as `/dev/ttyACM0` on Linux)

Make sure your shell has the Xtensa toolchain in scope before building the firmware:

```bash
. ~/export-esp.sh
```

---

## Firmware (`firmware/`)

### Flash and run

```bash
cargo run -p cubemaster-firmware --release --target xtensa-esp32s3-none-elf
```

This invokes the runner configured in `firmware/.cargo/config.toml`:

```
espflash flash --before usb-reset --after watchdog-reset
```

It flashes the bootloader, partition table, and app, then resets the chip into the application. It does not open a serial monitor.

### View logs (defmt over UART)

In a **separate terminal**, after flashing:

```bash
espflash monitor \
  --log-format defmt \
  --elf target/xtensa-esp32s3-none-elf/release/cubemaster \
  --non-interactive
```

You should see:

```
[INFO ] Embassy initialized! (cubemaster firmware/src/bin/main.rs:55)
[INFO ] Hello world! (cubemaster firmware/src/bin/main.rs:71)
```

Press `Ctrl+C` to stop the monitor.

### Why `--non-interactive`?

Without `--non-interactive`, `espflash monitor` enters an interactive mode that sets up a terminal raw-mode input reader. On some Linux hosts this fails with an I/O error because `ModemManager` races the port open on `/dev/ttyACM*`. `--non-interactive` skips that reader and reads the serial stream straight through.

### Quick one-liner (flash + monitor)

```bash
cargo run -p cubemaster-firmware --release --target xtensa-esp32s3-none-elf && \
  espflash monitor \
    --log-format defmt \
    --elf target/xtensa-esp32s3-none-elf/release/cubemaster \
    --non-interactive
```

### Firmware-specific files

- `firmware/src/bin/main.rs` — entry point (`#[esp_rtos::main]`), Embassy + BLE/WiFi init
- `firmware/.cargo/config.toml` — xtensa target runner, linker flags, build-std
- `firmware/build.rs` — linker scripts (`linkall.x`, `defmt.x`) + error helper
- `firmware/.clippy.toml` — stack-size lint threshold

### Logging stack

- `defmt` — deferred formatting macros (`info!`, `warn!`, ...)
- `esp-println` with `defmt-espflash` — ships defmt frames over UART0
- `esp-backtrace` — panic backtrace over defmt

---

## Companion app (`companion/`)

Dioxus desktop application for managing sound banks, controlling the cube via BLE/Wi-Fi, and configuring keyword-to-sound mappings.

### System dependencies (Linux)

```bash
sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev libsoup-3.0-dev \
                 libjavascriptcoregtk-4.1-dev libatk1.0-dev \
                 libgdk-pixbuf-2.0-dev libcairo2-dev libpango1.0-dev
```

### Build and run

No Xtensa toolchain needed — the companion app targets the host:

```bash
cargo run -p cubemaster-companion
```

First build fetches ~600 crates (Dioxus + WebView stack), subsequent builds are fast.

### Hot reload (development)

```bash
dx serve -p cubemaster-companion
```

Requires `dioxus-cli`: `cargo install dioxus-cli`.

---

## Shared crate (`shared/`)

Protocol types and constants shared between firmware and companion app.
See `shared/README.md` for module layout and design guidelines.

```bash
cargo check -p cubemaster-shared
```

---

## Docs (`doc/`)

Bikeshed engineering specification with PlantUML diagrams.

```bash
task spec          # compile .bs → .html
task serve         # build and open in browser
task clean         # remove generated files
```

Requires: `pipx install bikeshed`.

---

## CI

- **Rust CI** (`.github/workflows/rust_ci.yml`) — runs `cargo build`, `cargo fmt`, `cargo clippy` on push to main when Rust files change
- **Docs deploy** (`.github/workflows/deploy-docs.yml`) — builds the Bikeshed spec and deploys to GitHub Pages on push to main

---

## Troubleshooting

- **`cargo run -p cubemaster-firmware` fails to connect to `/dev/ttyACM0`**: unplug/replug the ESP32-S3, wait for `ModemManager` to finish probing, retry. Check `fuser /dev/ttyACM0` for competing processes (VSCode serial monitor, `screen`, another `espflash monitor`).
- **Garbled monitor output**: confirm `--log-format defmt` and `--elf` pointing to the correct ELF.
- **Chip stuck in download mode**: `--before usb-reset --after watchdog-reset` in the runner handles this for S3's USB-JTAG-Serial.
- **Companion app build fails with `pkg-config` errors**: install the GTK3/WebKit system deps listed above.
- **`dx serve` not found**: install `dioxus-cli` via `cargo install dioxus-cli`.
