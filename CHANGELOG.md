# Changelog

All notable changes to Resonance are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) · Versioning: [SemVer](https://semver.org/).

## [Unreleased]

### Added

- Project charter: README, architecture (control planes: Tuner / Conductor / Chamber), phased roadmap (0–5)
- ADR-0001 driver-first super-resolution strategy; ADR-0002 tech stack (Rust · Tauri 2 · Svelte 5)
- **Phase 0** — `drs-probe` PoC: NVAPI DRS runtime discovery, mode enum/switch, per-monitor DPI; full super-resolution chain proven on RTX 3060 Ti
- **Phase 1** — core engine: `resonance-core` (versioned config, crash-safe revert guard, harmonic profiles, rule model), `tuner` (display + DPI planes, guarded apply/revert, capability doctor), `conductor` (priority rules engine), `resctl` CLI; CI (fmt · clippy · test · release build)
- **Phase 2** — `chamber`: Tauri 2 + Svelte 5 tray-first control center with the harmonic-ring selector, crash-safe revert countdown overlay, typed IPC (`snapshot`/`apply`/`confirm`/`revert`), tray quick-switch, close-to-tray, single-instance
- **Phase 3** — automation: `conductor` watchers (foreground app / power / process list) and a tested action-emitting `Engine`; Chamber automation loop with manual-pin precedence, global hotkeys (panic-to-native `Ctrl+Alt+Shift+R`, cycle `Ctrl+Alt+R`), and an Automation tab (master toggle, live "why", rules list, add/remove rule)
- **Phase 4** — vendor layer: GPU vendor detection (NVIDIA/AMD/Intel) with vendor-specific "how to enable super-resolution" guidance; `doctor` and Chamber report vendor + NVAPI presence; documented that the core chain is vendor-agnostic (works for any GPU exposing above-native modes)
- **Phase 5** (in progress) — release engineering: NSIS installer via Tauri bundler (verified, ~3 MB); tag-triggered release workflow; GitHub Pages landing site (`site/`); split CI (fast checks on push, installer on tags); `CONTRIBUTING.md`; README capability matrix + screenshots + build-from-source quick start
