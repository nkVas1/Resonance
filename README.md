<div align="center">

# Resonance

**Super-resolution for your entire desktop.**
*RESO·lution·NANCE — render beyond your panel, everywhere, not just in games.*

[![Status](https://img.shields.io/badge/status-phase_0_·_research-8A2BE2)](docs/ROADMAP.md)
[![Platform](https://img.shields.io/badge/platform-Windows_11-0078D6?logo=windows)](#requirements)
[![Stack](https://img.shields.io/badge/stack-Rust_·_Tauri_2_·_Svelte_5-FF6B35)](docs/adr/0002-tech-stack.md)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

</div>

---

## What is this?

Your GPU can render the whole OS at a resolution **higher than your monitor supports** — 2880×1620 or 3840×2160 on a 1080p panel — and downsample it back with a high-quality (even AI-powered) filter. The result is dramatically sharper text, cleaner edges and richer detail in *every* application: browser, IDE, photo editors, video, desktop itself.

The technology already ships inside GPU drivers (NVIDIA **DSR / DLDSR**, AMD **VSR**), but it is buried in control panels, has zero automation, breaks DPI scaling, and nobody uses it outside games.

**Resonance** turns it into a first-class experience:

- 🔆 **One click / hotkey** to shift the entire desktop into super-resolution and back.
- 🎚️ **Automatic DPI compensation** — text stays physically the same size, just sharper (via per-monitor DPI control).
- 🤖 **Per-app automation** — Photoshop in the foreground? 2.25× DLDSR kicks in. Game closed? Native restored. On battery? Stay native.
- 🛟 **Safety rails** — every switch auto-reverts unless confirmed; no black-screen dead ends.
- 🖥️ **Tray-first UX** with a designed, animated control center — not another gray settings dialog.

## How it works

```
┌─────────────────────────────────────────────────────────────┐
│  OS composites everything at 2880×1620 / 3840×2160 (virtual) │
│      desktop · browser · apps · games — all supersampled     │
└──────────────────────────┬──────────────────────────────────┘
                           │  GPU driver downsample
                           │  (13-tap Gaussian / DL filter)
┌──────────────────────────▼──────────────────────────────────┐
│              Physical panel @ native 1920×1080               │
└─────────────────────────────────────────────────────────────┘
```

Resonance drives three control planes in concert:

| Plane | Component | Mechanism |
|---|---|---|
| Driver super-resolution | **Tuner** | NVAPI DRS (DSR/DLDSR factors & smoothness); AMD ADLX VSR planned |
| Display mode & safety | **Tuner** | Win32 `ChangeDisplaySettingsEx` / `SetDisplayConfig` + revert timer |
| DPI compensation | **Tuner** | Per-monitor DPI via `DisplayConfig(Get/Set)DeviceInfo` |
| Rules & automation | **Conductor** | Process/foreground/power watchers → declarative profiles |
| Control center UI | **Chamber** | Tauri 2 + Svelte 5, tray-first, motion-driven design |

Deep dive: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Why not just NVIDIA Control Panel / Magpie / Lossless Scaling?

- **NVIDIA Control Panel** exposes DSR as a raw checkbox: no desktop workflow, no DPI fix, no per-app logic, no hotkeys.
- **[Magpie](https://github.com/Blinue/Magpie)** and **Lossless Scaling** upscale *a single window* via capture — great for games, but they don't (and can't) raise the real desktop resolution system-wide.
- Resonance is **driver-first**: the OS genuinely renders more pixels, so *everything* benefits natively — no capture overhead, no cursor quirks, no per-window setup. Rationale: [ADR-0001](docs/adr/0001-driver-first-super-resolution.md).

## Requirements

- Windows 11 (Windows 10 21H2+ best-effort)
- NVIDIA GPU with DSR support (GTX 900+); DLDSR needs RTX (20-series+)
- AMD (VSR) and Intel support planned — see [roadmap](docs/ROADMAP.md)

## Status

**Phase 0 — research & proof-of-concept.** The full development plan lives in [docs/ROADMAP.md](docs/ROADMAP.md). Nothing installable yet — watch the repo.

## Project layout (planned)

```
crates/
  resonance-core/   # domain model, config, shared services
  tuner/            # NVAPI · display modes · DPI (unsafe FFI lives here only)
  conductor/        # rules engine, watchers, automation
apps/
  chamber/          # Tauri 2 + Svelte 5 control center (tray-first)
  resctl/           # CLI — everything scriptable
docs/               # architecture, roadmap, ADRs
```

## License

[MIT](LICENSE) © 2026 [nkVas1](https://github.com/nkVas1)
