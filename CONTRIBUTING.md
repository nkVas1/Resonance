# Contributing to Resonance

Thanks for your interest! Resonance is a Rust + Tauri + Svelte project. This guide gets you building and landing changes cleanly.

## Prerequisites

- **Rust** (stable) — <https://rustup.rs>
- **Node 22+** and **pnpm 11+** — `npm i -g pnpm`
- **Windows 11** with WebView2 (ships with the OS)
- An NVIDIA GPU with DSR available is ideal for testing the display planes, but the core builds and the CLI's `doctor` run on any machine.

## Repository layout

```
crates/
  resonance-core/   # domain model: config, profiles, rules, revert guard
  tuner/            # display + DPI control planes, vendor detection (unsafe FFI lives here)
  conductor/        # rules engine + system watchers
apps/
  resctl/           # CLI — every capability, scriptable
  chamber/          # Tauri 2 + Svelte 5 control center
poc/drs-probe/      # Phase 0 research tool (NVAPI/registry probing)
docs/               # architecture, roadmap, ADRs, research
site/               # landing page (GitHub Pages)
```

## Build & run

```bash
# Rust workspace (core, tuner, conductor, resctl)
cargo build
cargo run -p resctl -- doctor        # capability report for your machine

# Chamber (desktop app) — dev mode with hot reload
cd apps/chamber
pnpm install
pnpm tauri dev

# Production installer
pnpm tauri build                     # -> src-tauri/target/release/bundle/nsis/*-setup.exe
```

## Before you push

All of these run in CI and must pass:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps/chamber && pnpm check        # svelte-check (TS strict)
```

`unsafe` FFI is confined to `tuner` (display/DPI) and `conductor::watch` (process/power). Keep it there and wrap it in a safe API.

## Conventions

- **Commits:** [Conventional Commits](https://www.conventionalcommits.org) — `feat:`, `fix:`, `refactor:`, `docs:`, `perf:`, `chore:`. One logical change per commit.
- **Branches:** `feat/*`, `fix/*`; open PRs against `main`.
- **Tests:** add them where they protect real regression-prone logic (the rules engine and config migration are the sweet spots). Hardware planes are verified manually — see `docs/`.
- **Safety invariant:** any code path that changes the display mode must preserve the crash-safe revert guard. Never leave a switch unconfirmed without a timer.

## Reporting issues

Include your GPU + driver version, monitor native resolution, and the output of `resctl doctor`. For display glitches, note whether HDR or G-Sync was active.
