# Epic: Rust Workspace & Shared Infrastructure

**Epic ID:** EPIC-003
**Status:** Backlog
**Priority:** P0 — Critical
**Labels:** `infrastructure`, `workspace`, `ci`

## Goal

Set up the Cargo workspace monorepo structure with all crates, shared
protocol definitions, CI/CD pipelines, and developer tooling.

## Scope

| Task                       | Description                                    | Dependencies |
|----------------------------|------------------------------------------------|--------------|
| Workspace setup            | Root Cargo.toml with workspace members         | —            |
| Shared crate               | Protocol types, UUIDs, message encoding        | —            |
| Firmware skeleton          | Embassy project, xtensa target, build config   | esp-hal      |
| Companion app skeleton     | Dioxus project, platform feature flags         | dioxus      |
| x-tools skeleton           | CLI with clap, espflash integration            | clap         |
| CI pipeline                | Rust builds, clippy, fmt check, tests          | —            |
| Docs deployment            | Bikeshed + PlantUML → GitHub Pages             | —            |
| `.cargo/config.toml`       | Target config, linker, runner                  | —            |
| `rust-toolchain.toml`      | Pinned nightly version                         | —            |

## Acceptance Criteria

1. `cargo build --workspace` succeeds (firmware cross-compiles).
2. `shared/` crate compiles for both `no_std` and `std` targets.
3. `cargo fmt --check` and `cargo clippy --workspace` pass clean.
4. CI runs on every PR and blocks merge on failure.
5. Bikeshed spec deploys to `https://<org>.github.io/cubemaster/` on
   push to main.

## Child Tickets

- T-021: Create Cargo workspace with firmware, companion, shared, x-tools
- T-022: Define shared protocol types and message encoding
- T-023: Set up CI pipeline (build, clippy, fmt)
