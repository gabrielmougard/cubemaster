# Epic: MVP Companion App — Dioxus Cross-Platform Application

**Epic ID:** EPIC-002
**Status:** Backlog
**Priority:** P0 — Critical
**Labels:** `companion-app`, `dioxus`, `desktop`

## Goal

Deliver a Dioxus-based desktop companion application (Linux, macOS,
Windows) that can discover and connect to CubeMaster devices, manage
sound banks, upload files, configure keyword mappings, and trigger
sounds manually.

## Scope

| Feature                | Description                                          | Dependencies        |
|------------------------|------------------------------------------------------|---------------------|
| BLE client             | Scan, connect, GATT read/write/notify                | btleplug            |
| Wi-Fi upload client    | HTTP POST bulk upload, WebSocket streaming            | reqwest             |
| Sound bank manager     | Create/edit/import/export sound bank JSON manifests  | serde, shared crate |
| Cube connection UI     | Scan list, connect/disconnect, status dashboard      | dioxus              |
| Sound upload UI        | Select bank, progress bar, chunked upload            | dioxus              |
| Keyword config UI      | Map keywords to sounds, adjust thresholds            | dioxus              |
| Live control UI        | Trigger buttons, volume slider, stop all             | dioxus              |
| LED pattern preview    | Simulated cube face with lighting effects            | dioxus, canvas      |
| Local audio preview    | Play sound files locally before upload               | cpal                |
| Shared protocol crate  | BLE message types, UUIDs, message encoding           | shared crate        |

## Out of Scope

- Mobile builds (Android/iOS) — deferred to Post-MVP
- Sound pack marketplace / community hub
- Multi-cube management UI
- OTA firmware update UI

## Acceptance Criteria

1. App discovers CubeMaster devices via BLE scan and connects
   successfully.
2. Sound bank can be created from local audio files, exported as
   `.cubebank` file.
3. Sound bank uploads to cube over BLE (small files) and Wi-Fi
   (large files) with progress indication.
4. Keyword-to-sound mappings are configured on the cube via BLE and
   persist after power cycle.
5. Manual trigger buttons play the correct sound on the cube with
   synchronized LED patterns.
6. Volume slider adjusts cube volume in real-time over BLE.
7. LED pattern preview updates correctly in the UI.
8. App compiles and runs on at least Linux and macOS.

## Estimated Effort

8–10 weeks (solo developer)

## Child Tickets

See `tasks/tickets/`:
- T-012: Scaffold Dioxus desktop app with workspace integration
- T-013: Implement BLE client module (btleplug)
- T-014: Implement Wi-Fi upload client module
- T-015: Implement sound bank manager (CRUD + import/export)
- T-016: Implement cube connection and status UI
- T-017: Implement sound upload UI with progress
- T-018: Implement keyword configuration UI
- T-019: Implement live control UI
- T-020: Implement LED pattern preview canvas
