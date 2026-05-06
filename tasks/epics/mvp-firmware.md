# Epic: MVP Firmware — Cube Hardware & Embedded Software

**Epic ID:** EPIC-001
**Status:** Backlog
**Priority:** P0 — Critical
**Labels:** `firmware`, `esp32`, `embassy`

## Goal

Deliver the full CubeMaster firmware running on an ESP32-S3 dev kit on
a breadboard with all specified peripherals, powered via USB. The
firmware must accept commands over BLE/Wi-Fi, play sounds from SD,
drive LEDs, and perform on-device keyword spotting.

## Scope

| Module         | Description                                         | Dependencies    |
|----------------|-----------------------------------------------------|-----------------|
| LED driver     | 4× RMT DMA channels driving 16×16 WS2812B panels   | esp-hal, RMT    |
| Audio player   | I2S DMA output to MAX98357A, PCM 16kHz mono         | esp-hal, I2S    |
| Voice pipeline | PDM capture → WebRTC VAD → Goertzel KWS             | esp-hal, I2S PDM|
| BLE GATT       | 3 services (control, upload, patterns) via trouble-host | trouble-host |
| Wi-Fi server   | HTTP server + WebSocket on embassy-net               | embassy-net     |
| SD filesystem  | FAT32 via SPI, read/write/list                       | embedded-sdmmc  |
| Button handler | GPIO IRQ, debounce, press/hold detection             | esp-hal, GPIO   |
| Supervisor     | Embassy task orchestration, event dispatch           | all modules     |

## Out of Scope

- Battery charging circuit and BMS
- Power path management
- Multi-speaker spatial audio (single speaker for MVP)
- OTA firmware updates

## Acceptance Criteria

1. Cube advertises as BLE peripheral "CubeMaster-XXXX" and accepts
   GATT connections.
2. Sound files uploaded via BLE or Wi-Fi play correctly with <100ms
   latency.
3. Voice keyword "fire" triggers configured sound + LED pattern with
   ≥90% accuracy in quiet room.
4. All four LED panels refresh at 30 FPS without visible glitches.
5. Firmware binary <1MB flash, <100KB RAM peak.
6. All modules communicate via Embassy channels with no deadlocks.
7. BLE control works while Wi-Fi is uploading in parallel (dual-core).

## Estimated Effort

12–16 weeks (solo developer)

## Child Tickets

See `tasks/tickets/`:
- T-001: Set up Embassy project skeleton and toolchain
- T-002: Implement and test LED RMT DMA driver
- T-003: Implement and test I2S audio playback
- T-004: Implement SD card filesystem (FAT32 + SPI)
- T-005: Implement BLE GATT server (3 services)
- T-006: Implement Wi-Fi HTTP server + WebSocket
- T-007: Implement PDM capture + WebRTC VAD
- T-008: Implement Goertzel keyword spotter
- T-009: Implement button handler
- T-010: Implement main supervisor and event dispatch
- T-011: Integration test and end-to-end demo
