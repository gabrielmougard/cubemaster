# Cubemaster Dev Guide (ESP32-S3 + Rust)

A beginner-friendly guide to building, flashing, debugging, and iterating on this project. Target hardware: **ESP32-S3** connected over USB.

This project was bootstrapped with [`esp-generate`](https://github.com/esp-rs/esp-generate) and uses the [`esp-hal`](https://github.com/esp-rs/esp-hal) 1.x ecosystem with `embassy` async, `defmt` logging, and `probe-rs` as the flash/run tool.

---

## 1. One-time setup

### 1.1 Install the Xtensa Rust toolchain

The ESP32-S3 uses Xtensa cores, which are **not** supported by upstream `rustc`. You need Espressif's fork, installed via `espup`:

```bash
cargo install espup --locked
espup install
```

This produces a shell script you must source before every build (or add to your shell rc):

```bash
# after `espup install`, a file is created at:
source ~/export-esp.sh
```

Add this to your `~/.zshrc` so you don't forget:

```bash
echo 'source ~/export-esp.sh' >> ~/.zshrc
```

Verify:

```bash
rustup toolchain list        # should list `esp`
rustc +esp --version         # should print a version
```

> The file `rust-toolchain.toml` has `channel = ""` (empty). That's intentional for Xtensa — the `esp` toolchain is picked up from the `espup` environment, not from `rustup`'s usual mechanism. Don't worry about it.

### 1.2 Install the flash/run/debug tools

This project's `.cargo/config.toml` already sets:

```toml
runner = "probe-rs run --chip=esp32s3 --preverify --always-print-stacktrace --no-location"
```

So `cargo run` will invoke `probe-rs`. Install it:

```bash
cargo install probe-rs-tools --locked
```

Also install `espflash` as a fallback (sometimes easier for first-time flashing and for monitor-only sessions):

```bash
cargo install espflash --locked
```

On macOS you usually **don't need udev rules**, but if `probe-rs` can't see the device, see §6 (Troubleshooting).

### 1.3 USB driver / connection mode (ESP32-S3 specifics)

The ESP32-S3 has **two** possible USB paths:

1. **Native USB-Serial/JTAG** (built into the chip, pins GPIO19/GPIO20). No external USB-UART chip needed. `probe-rs` talks to this.
2. **UART-over-USB** via a CP210x / CH340 / FTDI chip on the dev board.

Most modern S3 dev boards (e.g. ESP32-S3-DevKitC-1, ESP32-S3-WROOM) expose **both** ports, appearing as two serial devices when plugged in. You want the **USB-Serial/JTAG** one for `probe-rs`.

Check what's connected:

```bash
ls /dev/tty.* /dev/cu.*
```

Typical names on macOS:
- `/dev/cu.usbmodem*` → native USB-Serial/JTAG (use this with `probe-rs`)
- `/dev/cu.usbserial-*` or `/dev/cu.SLAB_USBtoUART` → UART bridge (use this with `espflash` or a plain serial monitor)

List probes `probe-rs` can see:

```bash
probe-rs list
```

You should see an entry like `ESP32-S3 -- ... (USB JTAG/serial debug unit)`. If not, see §6.

---

## 2. Day-to-day: build, flash, run

Every new terminal:

```bash
source ~/export-esp.sh
```

Then, from the repo root:

### 2.1 Just build (no device needed)

```bash
cargo build              # debug build
cargo build --release    # optimized build
```

Artifacts land in `target/xtensa-esp32s3-none-elf/{debug,release}/cubemaster`.

### 2.2 Flash and run with live logs (the normal dev loop)

```bash
cargo run --release
```

What this does:
1. Compiles.
2. Invokes `probe-rs run --chip=esp32s3 ...` on the ELF.
3. Flashes the binary to the chip.
4. Resets the chip and streams `defmt` logs back over RTT to your terminal.

You should see:

```
INFO  Embassy initialized!
INFO  Hello world!
INFO  Hello world!
...
```

Hit `Ctrl-C` to stop. Edit code, run again. That's the loop.

> `--release` is strongly recommended. Debug builds for embedded Xtensa are *huge* and slow; the project's `Cargo.toml` even sets `opt-level = "s"` for `[profile.dev]` for that reason. Still, release is smaller and faster.

### 2.3 Change log verbosity

`DEFMT_LOG` is set to `info` in `.cargo/config.toml`. Override per-run:

```bash
DEFMT_LOG=debug cargo run --release
DEFMT_LOG=trace cargo run --release
DEFMT_LOG=warn  cargo run --release
```

You can also scope by module: `DEFMT_LOG=cubemaster=trace,esp_hal=info`.

### 2.4 Run the on-device tests

This repo uses [`embedded-test`](https://github.com/probe-rs/embedded-test). Tests live in `tests/` and actually run on the chip:

```bash
cargo test --release
```

The demo test in `tests/hello_test.rs:28` (`hello_test`) will flash, run, assert, and report back. You need the device connected for this.

---

## 3. Alternative: flashing via UART with `espflash`

If `probe-rs` won't cooperate (rare on S3 native USB, common on older boards), you can use `espflash` which talks to the ROM bootloader over UART:

```bash
# Flash and monitor
cargo build --release
espflash flash --monitor target/xtensa-esp32s3-none-elf/release/cubemaster

# Or just monitor logs from an already-flashed chip
espflash monitor
```

To set `espflash` as the Cargo runner instead of `probe-rs`, edit `.cargo/config.toml`:

```toml
[target.xtensa-esp32s3-none-elf]
runner = "espflash flash --monitor"
```

> Note: `espflash` won't give you `defmt` decoding over RTT — you'd see raw bytes. For `defmt` logs via UART you'd need additional setup. The `probe-rs` path (the default) is the smoother experience for this project.

---

## 4. Putting the chip into bootloader mode (when needed)

`probe-rs` and modern `espflash` can usually reset the chip into download mode automatically. But if flashing fails with "chip not responding":

1. Hold the **BOOT** button (sometimes labeled `IO0`).
2. Press and release **RESET** (sometimes `EN`).
3. Release **BOOT**.
4. Re-run `cargo run --release`.

After flashing, press **RESET** once to start the new firmware.

---

## 5. Editor & IDE setup

### VS Code / Cursor

Install:
- **rust-analyzer** extension.
- **probe-rs debugger** extension (for breakpoint debugging — the `esp-generate` `-o vscode` flag you used should have created a `.vscode/launch.json`).

Tell rust-analyzer to use the `esp` toolchain:

`.vscode/settings.json`:
```json
{
  "rust-analyzer.cargo.target": "xtensa-esp32s3-none-elf",
  "rust-analyzer.check.allTargets": false
}
```

> rust-analyzer may complain it can't find the Xtensa target. Make sure VS Code was launched from a terminal that sourced `~/export-esp.sh`, or add the env export to its settings.

### CLI quality-of-life

```bash
cargo install cargo-watch
cargo watch -x 'run --release'    # re-flash on every save
```

---

## 6. Troubleshooting

### "error: component 'rust-std' for target 'xtensa-esp32s3-none-elf' is unavailable"
You forgot `source ~/export-esp.sh`. The `esp` toolchain isn't active.

### `probe-rs list` shows nothing
- Are you plugged into the **USB** port (native) and not only the **UART** port? On dual-port boards, try the other USB-C / micro-USB jack.
- Is another process holding the port? (`lsof | grep usbmodem`)
- macOS sometimes needs the cable re-seated after `espup install` adjusts udev-ish rules.
- Try `probe-rs info --chip esp32s3`.

### `Error: The flashing procedure failed`
- Put the chip in bootloader mode manually (§4).
- Lower the baud rate: `probe-rs run --chip=esp32s3 --speed 1000 ...` (edit the runner in `.cargo/config.toml`).

### `defmt` shows garbled output or version errors
- The `defmt` crate version must match what `probe-rs` expects. Keep them updated: `cargo install probe-rs-tools --locked --force`.

### Linker errors mentioning `_defmt_*`, `_stack_start`, `esp_rtos_*`
The `build.rs` in this repo (see `build.rs:9`) prints a hint for each. Read the message — it usually tells you exactly which linker script or dependency is missing.

### "signal: 11, SIGSEGV: invalid memory reference" while building
Out-of-memory during LTO. Either close other apps, or temporarily disable `lto = 'fat'` in `Cargo.toml` `[profile.release]`.

### Build is painfully slow
- First build always is (compiling `esp-hal`, `embassy`, `smoltcp`, `trouble-host`, etc.). Subsequent incremental builds are fast.
- Use `cargo build --release` rather than `cargo clean && build` — the build cache is your friend.

---

## 7. Project layout cheat sheet

```
cubemaster/
├── Cargo.toml             # deps + profiles
├── .cargo/config.toml     # target + runner + rustflags
├── rust-toolchain.toml    # (uses `esp` toolchain via espup env)
├── build.rs               # linker script glue + friendlier linker errors
├── src/
│   ├── bin/main.rs        # firmware entry point
│   └── lib.rs             # (empty) for shared code
└── tests/
    └── hello_test.rs      # on-device test with embedded-test
```

Entry point: `src/bin/main.rs:38` (the `#[esp_rtos::main]` async `main`).

---

## 8. Typical iteration workflow

```bash
# once per terminal
source ~/export-esp.sh

# tight loop
cargo run --release              # build + flash + logs
# edit code...
cargo run --release              # again

# or automatic
cargo watch -x 'run --release'

# occasionally
cargo test --release             # on-device tests
cargo clippy --release           # lints
cargo fmt                        # formatting
```

That's it. Happy hacking.
