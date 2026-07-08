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
