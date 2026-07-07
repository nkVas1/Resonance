# Resonance — Roadmap

> Phases follow the "runnable at every boundary" rule: each phase ends with something you can launch and use. Estimates assume part-time development; treat them as sequencing, not deadlines.

## Phase 0 — Research & Proof of Concept 🔬 *(current)*

Goal: prove the entire driver-first chain end-to-end on real hardware (RTX 3060 Ti, 1080p panel) before writing a single line of product code.

- [ ] `poc/` Rust workspace: link `nvapi64.dll`, create DRS session, dump all settings (`DRS_EnumSettings`) to identify DSR/DLDSR IDs and value encodings on current driver
- [ ] Programmatically enable DLDSR 2.25× + DSR 4× (+ smoothness), verify new modes appear in `EnumDisplaySettingsEx`
- [ ] Switch desktop to 2880×1620 and back via `ChangeDisplaySettingsEx`; measure switch latency
- [ ] Set per-monitor DPI via `DisplayConfigSetDeviceInfo(-4)` with read-back verification
- [ ] Full cycle demo: native → DLDSR + DPI 150% → revert, from one CLI command
- [ ] Findings written up in `docs/research/phase0-findings.md` (IDs, encodings, latencies, gotchas: HDR, G-Sync, multi-monitor)

**Exit criteria:** one-command supersampled desktop on the dev machine, documented.

## Phase 1 — Core Engine ⚙️

Goal: production-quality Rust core + CLI. Everything Resonance will ever do, scriptable.

- [ ] Cargo workspace: `resonance-core`, `tuner`, `conductor` (stub), `resctl`
- [ ] Safe NVAPI wrapper (`unsafe` confined to `tuner::ffi`), runtime setting-ID discovery with fallback table
- [ ] Display topology model + mode switching + **persisted revert timer** + panic restore
- [ ] DPI compensation plane with canary validation
- [ ] Config store (versioned TOML, `%APPDATA%/Resonance`), `tracing` logging
- [ ] `resctl` CLI: `status`, `enable`, `switch <profile>`, `revert`, `doctor` (capability probe & diagnostics)
- [ ] CI: GitHub Actions — fmt, clippy `-D warnings`, build; release-profile artifact
- [ ] Unit tests: config migration, profile resolution logic (only pure-logic surfaces)
- [ ] `docs/VERIFY.md` — manual hardware verification checklist

**Exit criteria:** `resctl switch dldsr-2.25x` works reliably with safety revert; `resctl doctor` explains capability on any machine.

## Phase 2 — Chamber: App Shell & UI 🎨

Goal: the designed, animated control center. First public screenshots.

- [ ] Tauri 2 scaffold, single-instance, tray-first lifecycle (close-to-tray, autostart toggle)
- [ ] Design system: tokens (color/type/motion), standing-wave motif, harmonic-ring resolution selector, spring micro-animations, `prefers-reduced-motion`
- [ ] Onboarding wizard: detection → capability card → guided first switch with live countdown → DPI calibration preview
- [ ] Dashboard: current state, one-click profiles, smoothness slider with preview
- [ ] Global hotkeys + tray quick-switch
- [ ] Typed IPC layer (`state/changed` event, command handlers) — UI as pure function of state
- [ ] README screenshots + short demo GIF; `design-archive/` established

**Exit criteria:** a non-technical user can install a dev build, run the wizard, and live in super-resolution daily.

## Phase 3 — Conductor: Automation 🤖

Goal: Resonance manages itself; the user stops thinking about it.

- [ ] Rules engine (priority, first-match, debounced) + TOML rule schema
- [ ] Watchers: process start/stop (WMI), foreground app (WinEvent hook), power source
- [ ] Per-app profiles UI: searchable process picker, "why is my resolution X" inspector
- [ ] Restore semantics: on-exit / on-background / manual pin
- [ ] Profile import/export (single file, shareable)
- [ ] Unit tests: rules engine (this is exactly where tests pay off)

**Exit criteria:** demo scenario — open Photoshop → DLDSR engages; switch to battery → native; close app → restored. Zero manual input.

## Phase 4 — Beyond NVIDIA 🧩

Goal: widen hardware support; add the capture plane as a complement, not a rewrite.

- [ ] `SuperResProvider` for AMD **VSR** via ADLX
- [ ] Intel best-effort via IGCL (custom scaled modes where possible)
- [ ] Evaluate capture-upscale plane for unsupported hardware: integrate/hand-off to [Magpie](https://github.com/Blinue/Magpie) vs. minimal in-house FSR1 pipeline (Windows.Graphics.Capture + D3D11 compute) — decision recorded as ADR-0003
- [ ] Multi-monitor mixed topologies (per-display profiles)

**Exit criteria:** meaningful feature set on at least one non-NVIDIA test config; clear capability matrix in README.

## Phase 5 — Release & Polish 🚀

Goal: v1.0 on GitHub Releases; installable in one step, updatable in zero.

- [ ] Installer via Tauri bundler (NSIS) + `winget` manifest
- [ ] Opt-in auto-update (Tauri updater, GitHub Releases channel)
- [ ] RU localization (Fluent), accessibility pass (keyboard nav, contrast, reduced motion)
- [ ] Performance pass: idle footprint budget (< 30 MB RAM, ~0% CPU), cold-start < 1 s to tray
- [ ] Docs site (GitHub Pages): user guide, FAQ (HDR, laptops, text softness), troubleshooting
- [ ] `CONTRIBUTING.md`, issue templates, v1.0.0 tag + announcement post

**Exit criteria:** a stranger with an RTX card goes from README to supersampled desktop in under 3 minutes.

---

## Non-goals (v1)

- Frame generation / latency-sensitive game overlays (Lossless Scaling's territory)
- Linux/macOS (driver APIs are Windows-specific; revisit post-v1)
- Any cloud service, account, or telemetry

## Versioning

SemVer. `main` always releasable after Phase 1; feature branches `feat/*`, fixes `fix/*`. Changes tracked in [CHANGELOG.md](../CHANGELOG.md) (Keep a Changelog).
