# ADR-0002: Tech stack — Rust core, Tauri 2 shell, Svelte 5 UI

- **Status:** accepted · 2026-07-08 (confirmed with project owner)
- **Context:** The product needs (a) low-level Win32/NVAPI FFI with strong safety guarantees, (b) a tray-resident app with a tiny idle footprint, (c) a designer-grade UI with rich micro-animations. Candidates: Tauri 2 + Rust + Svelte 5; WinUI 3 + .NET 9; pure-Rust UI (Slint/egui); Electron.
- **Decision:** **Rust** for all system layers (`resonance-core`, `tuner`, `conductor`) — memory safety around `unsafe` FFI, `windows` crate for Win32, zero-cost abstractions. **Tauri 2** for the shell — native WebView2, ~10 MB binaries, first-class tray/single-instance/updater. **Svelte 5** (runes) + Motion One for the UI — compile-time reactivity (no VDOM cost on an always-running tray app) and full CSS/JS freedom for the standing-wave design language.
- **Alternatives rejected:**
  - *WinUI 3 / .NET:* easier P/Invoke, but Fluent constrains the visual identity and heavy customization fights the framework; larger runtime.
  - *Slint/egui:* smallest footprint, but the design ceiling (typography, motion, layout finesse) is far lower — UI is a core differentiator here.
  - *Electron:* footprint and memory unacceptable for a tray-resident utility.
- **Consequences:** two toolchains (Rust + Node) in CI; WebView2 dependency (ships with Windows 11 — acceptable); all OS mutations stay in Rust, the UI is a pure state renderer over typed IPC.
