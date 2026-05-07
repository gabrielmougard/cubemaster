## Can the ESP32-S3 drive 4× WS2812B 16×16 matrices directly? **Yes.**

We do **not** need a GLEDOPTO or any dedicated controller board. The ESP32-S3 is more than capable of driving 4 parallel WS2812B outputs directly from its GPIO pins. The GLEDOPTO is literally just an ESP32 + level shifter + power connectors in a box.

### What makes it work

The ESP32-S3 has hardware peripherals perfect for this:

- **RMT peripheral** : 4 TX channels, each can bit-bang WS2812B timing in hardware
- **I2S parallel mode** : can drive 8 or 16 strips in true parallel (useful for large installs)
- **SPI** : another option via DMA

### The numbers for our setup

| Spec | Value |
|---|---|
| LEDs per matrix | 256 |
| Total LEDs | 1,024 |
| Max current (all white, full brightness) | ~60 mA × 1024 ≈ **61 A @ 5V** |
| Realistic typical current | ~10–20 A (animations rarely hit full white) |
| Data rate per strip | 800 kHz |
| Refresh time for 256 LEDs | ~7.7 ms (≈130 FPS ceiling) |

<details>
<summary><strong>Hardware checklist (click to expand)</strong></summary>

1. **Power supply** : A 5V / 20A PSU is a reasonable minimum. **We do not power the matrices from the ESP32's USB!**
2. **Power injection** : Inject 5V + GND at both ends of each matrix (and possibly mid-way) to avoid voltage drop / color shift on the far end.
3. **Level shifter (recommended)** : ESP32-S3 outputs 3.3V logic. WS2812B spec wants ≥0.7× VDD (3.5V at 5V). It often "just works" but is out of spec. We should use a **74AHCT125** or **SN74HCT245** to shift 3.3V -> 5V for reliability. We should keep the first LED physically close to the shifter.
4. **Capacitor** : 1000 µF across 5V/GND at the power input of each matrix.
5. **Resistor** : 330–470 Ω inline on each data line, close to the first LED.
6. **Common ground** : ESP32 GND must connect to PSU GND.

</details>

# Power supply and BOM for 4× 16×16 WS2812B matrices

## Choosing the 5V PSU

Worst-case load is ~61 A, but we'll never hit that in practice. Here's how to size it:

| Use case | Recommended PSU | Why |
|---|---|---|
| Global brightness capped at ~30%, colorful animations | **5V / 20A (100W)** | Typical real-world draw; most common choice |
| Brightness up to ~60%, occasional white flashes | **5V / 40A (200W)** | Safe headroom, handles surges |
| Full brightness white, no compromises | **5V / 60A (300W)** | True worst-case |

> **Rule of thumb:** size for ~60% of theoretical max. For 1,024 LEDs, a **5V 40A Mean Well LRS-200-5** is the sweet spot for most projects.

## Recommended PSU options (in order of preference)

1. **Mean Well LRS-200-5** : 5V / 40A / 200W -> *gold standard, ~$35, safety certified (UL/TÜV), quiet fanless design*
2. **Mean Well LRS-350-5** : 5V / 60A / 350W -> *if we want zero compromises, ~$50*
3. **Mean Well RS-150-5** : 5V / 26A / 130W -> *budget option if we'll cap brightness*

## Full BOM

<details open>
<summary><strong>Core electronics</strong></summary>

| # | Qty | Part | Purpose | Approx. price |
|---|---|---|---|---|
| 1 | 1 | **Mean Well LRS-200-5** (5V 40A) | Main power | $35 |
| 2 | 1 | **ESP32-S3 DevKitC-1** (N16R8 preferred) | Controller | $10–15 |
| 3 | 1 | **74AHCT125** DIP-14 (or SN74HCT245) | 3.3V → 5V level shifter | $1 |
| 4 | 1 | 14-pin DIP socket | For the 74AHCT125 | $0.30 |
| 5 | 4 | **470 Ω resistor**, ¼W | Data line series resistors | $0.10 |
| 6 | 4 | **1000 µF / 10V or 16V** electrolytic capacitor | Bulk cap at each matrix input | $2 |
| 7 | 1 | **0.1 µF ceramic capacitor** | Decoupling near 74AHCT125 VCC | $0.10 |
| 8 | 1 | **C14 IEC inlet with fused switch** | Mains power entry (safety!) | $5 |
| 9 | 1 | **5A slow-blow fuse** (for IEC inlet) | Mains-side protection | $1 |
| 10 | 4 | **5A automotive blade fuse + holder** | Per-matrix 5V protection | $5 |

</details>

<details open>
<summary><strong>Wiring and connectors</strong></summary>

| # | Qty | Part | Purpose |
|---|---|---|---|
| 11 | ~2 m | **18 AWG silicone wire** red + black | 5V distribution from PSU to matrices |
| 12 | ~1 m | **22 AWG hookup wire** assorted | Data lines, logic |
| 13 | 1 set | **WAGO 221 lever nuts** (221-413, 3-conductor, 5-pack) | Clean 5V/GND splicing |
| 14 | 4 | **JST-SM 3-pin** pigtails (if your matrices have them) | Matrix data connectors |
| 15 | 1 | Breadboard + jumper wires | Prototyping |

</details>

<details>
<summary><strong>Optional but recommended</strong></summary>

| # | Qty | Part | Purpose |
|---|---|---|---|
| 16 | 1 | **USB-C 5V 3A buck converter** OR use ESP32 USB | Logic power (keep separate from LED rail during dev) |
| 17 | 1 | **Multimeter** | Verify PSU voltage is ~5.0V before connecting LEDs |
| 18 | 1 | Project enclosure (vented) | Safety, especially for mains-side |
| 19 | 1 roll | Heat-shrink tubing, assorted | Insulate mains and high-current joints |
| 20 | 1 | **P-channel MOSFET** (e.g., AO3401) + gate resistor | Optional: master on/off via GPIO |

</details>

## Wiring diagram (text)

```
 Mains AC ──► [C14 inlet + 5A fuse + switch] ──► [LRS-200-5]
                                                      │
                                                   5V │ GND
                                                      ▼
                                              [WAGO splice block]
                                               │   │   │   │
                                            [5A][5A][5A][5A]  ← blade fuses
                                               │   │   │   │
                                              M1  M2  M3  M4  (5V + GND to each matrix,
                                                               inject at both ends if possible)

 ESP32-S3 ──► GPIO ──► [470Ω] ──► 74AHCT125 input (Vcc=5V from PSU)
                                      │
                                      ├──► Matrix 1 DIN
                                      ├──► Matrix 2 DIN
                                      ├──► Matrix 3 DIN
                                      └──► Matrix 4 DIN

 **ESP32 GND ↔ PSU GND ↔ Matrix GND — all tied together**
 1000 µF cap across 5V/GND at each matrix input
```

## Critical safety notes

> **Mains wiring**: If we're not comfortable with AC mains (the terminals on the LRS-200-5 are exposed), use a pre-made **IEC C14 inlet module with integrated fuse and switch**, or buy a PSU with a built-in AC cord. Always enclose the PSU's AC terminals. Anyway in the real cube, if we decide to embed a transformer, we'll need such an inlet.

> **First power-on**: Before connecting any LEDs, power up the PSU alone and verify output is **4.95–5.10V** with a multimeter. Adjust the trim pot on the PSU if needed (LRS-series has one near the output terminals).

> **Do not back-power the ESP32**: While developing, keep the ESP32 on USB power. Once standalone, power it via a separate buck converter or the 5V rail, but never have both USB and 5V rail connected simultaneously without a diode/switch.

## Estimated total: **~$70–80**

That's roughly the price of the GLEDOPTO alone, and we end up with a more flexible, higher-capacity, fully-custom rig.

# Wiring 4× WS2812B 16×16 Matrices

Our matrix has the standard layout:

| Side | Cable | Purpose |
|---|---|---|
| **Left (3-wire)** | 5V / GND / **DOUT** | Data **output** — feeds the *next* panel's DIN. Also power pass-through. |
| **Middle (2-wire)** | 5V / GND | **Power injection** — pure power, no data. Lets you feed 5V directly to the middle of the panel to avoid voltage drop. |
| **Right (3-wire)** | 5V / GND / **DIN** | Data **input** — this is where data comes *in* from the ESP32 (or previous panel). |

> **Key insight:** Data flows **right -> left** on a single panel (DIN on right, DOUT on left). The middle 2-wire cable is *only* for power, it carries no data signal.

## Two wiring strategies (we should pick one)

### Strategy A: 4 independent data lines (RECOMMENDED)

Each matrix gets its **own GPIO** from the ESP32. This is what our original plan implied, and it's the best choice because:

- **4× faster refresh** (each panel only has 256 LEDs of latency, not 1024)
- **If one panel fails, the others keep working**
- **Simpler software mapping** per panel
- **Matches the 4-output GLEDOPTO model**

With this method, we **only use the right-side DIN cable** on each panel. The left-side DOUT stays unused (tape it off).

### Strategy B: All 4 chained on 1 data line

Connect panel 1's DOUT -> panel 2's DIN -> panel 3's DIN -> panel 4's DIN. Uses only 1 GPIO, but slower refresh and one failure kills the chain. **Not recommended** for 1,024 LEDs.

---

## Full wiring for Strategy A (4 parallel panels)

### Power distribution (shared across all 4 panels)

Every panel gets **two 5V feeds and two GND feeds** : one pair on the right-side connector, one pair on the middle injection connector. This is power injection done right.

```
                    ┌─────────────────────┐
 Mains AC ─────────►│   LRS-200-5 (5V)    │
                    └──┬───────────────┬──┘
                    5V │            GND│
                       ▼               ▼
                  [WAGO 5V bus]   [WAGO GND bus]
                      │                    │
        ┌──────┬──────┼──────┬──────┐  (same fan-out for GND)
        │      │      │      │      │
      [5A]   [5A]   [5A]   [5A]   fuses (optional but smart)
        │      │      │      │
        ▼      ▼      ▼      ▼
      Panel1 Panel2 Panel3 Panel4
```

### Per-panel wiring (do this 4 times)

For **each** matrix, connect:

| Matrix cable | Wire | Connects to |
|---|---|---|
| **Right** red | 5V | 5V bus (via fuse) |
| **Right** white | GND | GND bus |
| **Right** green | DIN | 74AHCT125 output (via 470 Ω resistor) |
| **Middle** red | 5V | 5V bus (same rail, second feed) |
| **Middle** black | GND | GND bus (second feed) |
| **Left** red | — | Leave disconnected (or tie to 5V as a 3rd injection point for extra safety) |
| **Left** white | — | Leave disconnected (or tie to GND) |
| **Left** green (DOUT) | — | **Leave disconnected and insulated** (tape it off) |

> Connecting both the right-side and middle 5V/GND to the same bus gives us **dual power injection per panel**. This prevents the far end of the panel from dimming or turning pinkish/red on bright scenes.

### Data lines from ESP32-S3

We must pick 4 safe GPIOs on the ESP32-S3. Good choices that avoid strapping/boot pins and PSRAM conflicts:

| Panel | ESP32-S3 GPIO |
|---|---|
| Panel 1 | **GPIO 4** |
| Panel 2 | **GPIO 5** |
| Panel 3 | **GPIO 6** |
| Panel 4 | **GPIO 7** |

Avoid: GPIO 0, 3, 45, 46 (strapping), 19/20 (USB), 26–32 (flash), 33–37 (octal PSRAM on N8R8/N16R8).

### Level shifter wiring (74AHCT125)

The '125 is a quad buffer which is perfect, one gate per panel. Pinout:

```
           ┌──── U ────┐
  1  ─OE1──┤1        14├── VCC ──► +5V from PSU
  2  ─IN1──┤2        13├── OE4 ──► GND
  3  ─OUT1─┤3        12├── IN4
  4  ─OE2──┤4        11├── OUT4
  5  ─IN2──┤5        10├── OE3
  6  ─OUT2─┤6         9├── IN3
  7  ─GND──┤7         8├── OUT3
           └───────────┘
```

Wiring table:

| 74AHCT125 pin | Connect to |
|---|---|
| 14 (VCC) | +5V rail |
| 7 (GND) | GND rail |
| 1, 4, 10, 13 (all OE) | **GND** (active-low enables, tie to ground to enable outputs) |
| 2 (IN1) | ESP32 GPIO 4 |
| 5 (IN2) | ESP32 GPIO 5 |
| 9 (IN3) | ESP32 GPIO 6 |
| 12 (IN4) | ESP32 GPIO 7 |
| 3 (OUT1) | 470 Ω → Panel 1 DIN (right-side green) |
| 6 (OUT2) | 470 Ω → Panel 2 DIN |
| 8 (OUT3) | 470 Ω → Panel 3 DIN |
| 11 (OUT4) | 470 Ω → Panel 4 DIN |

Also, we should add a **0.1 µF ceramic cap** between pin 14 and pin 7, right at the chip.

**Don't forget:** ESP32 GND <-> 74AHCT125 GND <-> PSU GND <-> all panel GNDs = **one common ground**.

## Complete wiring diagram

```
                                                     ┌──────────────┐
  ESP32-S3                                           │   5V / 40A   │
  ┌────────┐                                         │   LRS-200-5  │
  │  GPIO4 ├──────┐                                  └──┬────────┬──┘
  │  GPIO5 ├────┐ │                                   5V│     GND│
  │  GPIO6 ├──┐ │ │                                     ▼        ▼
  │  GPIO7 ├┐ │ │ │                                 ═══════════════════
  │   GND  ├┼─┼─┼─┼─────────────────────────────────┐  (common bus)
  │   5V   ├┤ │ │ │   ┌────────────┐                │
  └────────┘│ │ │ │   │ 74AHCT125  │                │
            │ │ │ │   │  VCC──5V───┼────────────────┤
            │ │ │ └──►│IN1  OUT1──►│─[470Ω]──► P1 DIN (right green)
            │ │ └────►│IN2  OUT2──►│─[470Ω]──► P2 DIN
            │ └──────►│IN3  OUT3──►│─[470Ω]──► P3 DIN
            └────────►│IN4  OUT4──►│─[470Ω]──► P4 DIN
                      │ OE1-4──GND │
                      │  GND───────┼────────────────┤
                      └────────────┘                │
                                                    │  Per panel:
                                                    │  ├─ Right red  → 5V
  [1000µF cap at each panel's power input]          │  ├─ Right white→ GND
                                                    │  ├─ Middle red → 5V
                                                    │  ├─ Middle blk → GND
                                                    │  └─ Left cables: UNUSED
                                                    └──
```

## Pre-flight checklist

1. **Verify DIN vs DOUT** : before wiring, power one panel alone with just the right-side cable and send data. If it doesn't light up, we may have DIN on the left instead (some vendors reverse it). Swap and retry.
2. **Voltage check** : PSU outputs 4.95–5.10V before any LED is connected.
3. **Common ground** : measure continuity between ESP32 GND and PSU GND.
4. **Start with one panel** : get one panel working perfectly before wiring the other three.
5. **Low brightness first** : set global brightness to ~10% for first power-on to avoid a 60A surprise.

## Quick sanity test

After wiring panel 1 only, run a test that lights **just the first LED red** at low brightness. If the **top-right** (or wherever DIN enters) lights up, we're good. If nothing lights, swap left and right 3-wire cables. Our panel may have the opposite convention.

