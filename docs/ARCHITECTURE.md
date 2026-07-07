# Resonance — Architecture

> Living document. Updated at phase boundaries, not per-commit.

## 1. Product thesis

GPU drivers already know how to render the OS above native panel resolution and downsample with high-quality filters (NVIDIA DSR 2014, DLDSR 2022, AMD VSR). What's missing is the *product* around it: safe switching, DPI compensation, automation, and a UX that makes super-resolution a daily tool instead of a hidden checkbox. Resonance supplies that product layer.

**Driver-first, capture-second** (see ADR-0001): we prefer making the OS *actually* render more pixels over screen-capture upscaling. Capture pipelines (Magpie-style) are a later, complementary plane for unsupported hardware — never the core.

## 2. System overview

```
┌───────────────────────────── Chamber (Tauri 2 shell) ─────────────────────────────┐
│  Svelte 5 UI · tray menu · onboarding wizard · dashboard · profile editor         │
│                          Tauri IPC (commands + typed events)                      │
└──────────────────────────────────────┬────────────────────────────────────────────┘
                                       │
┌──────────────────────────── resonance-core (Rust) ────────────────────────────────┐
│  DisplayTopology model · Profile/Rule schema · config store · tracing · errors    │
└───────────┬──────────────────────────────────────────────────────┬────────────────┘
            │                                                      │
┌───────────▼──────────── tuner ───────────────┐   ┌───────────────▼── conductor ───┐
│ NVAPI DRS (DSR/DLDSR factors, smoothness)    │   │ process watcher (WMI events)   │
│ mode switch (ChangeDisplaySettingsEx /       │   │ foreground hook (WinEvent)     │
│   SetDisplayConfig) + revert timer           │   │ power events (AC/battery)      │
│ per-monitor DPI (DisplayConfig ±3/±4)        │   │ hotkeys (RegisterHotKey)       │
│ vendor abstraction: Nvidia | Amd | Intel     │   │ rules engine → tuner actions   │
└──────────────────────────────────────────────┘   └────────────────────────────────┘
```

Single user-mode process, single instance, tray-resident. No Windows service, no elevation for the standard path (DRS writes, mode switches and DPI changes all work from user context; elevation is requested ad hoc only if a specific driver/OEM configuration demands it).

## 3. Control planes (crate: `tuner`)

### 3.1 Super-resolution plane — NVAPI DRS

- FFI to `nvapi64.dll` (`NvAPI_QueryInterface` → function pointers), wrapped in a **safe** Rust API; all `unsafe` is confined to `tuner::ffi`.
- Flow: `DRS_CreateSession → LoadSettings → GetBaseProfile → SetSetting(DSR factors, DSR smoothness) → SaveSettings`.
- DSR/DLDSR setting IDs are **undocumented**; they are recovered at runtime via `DRS_EnumSettings` name matching, cross-checked against the ID table extracted from [nvidiaProfileInspector](https://github.com/Orbmu2k/nvidiaProfileInspector). Unknown driver layouts degrade gracefully: feature reported "unavailable", never a blind write.
- DLDSR factors (1.78×, 2.25×) are encoded distinctly from classic DSR factors (bit-flagged values) — exact encoding is a Phase 0 deliverable, validated on RTX 3060 Ti / driver R575+.
- After `SaveSettings`, mode list refresh is verified via `EnumDisplaySettingsEx`; if new modes don't appear, we surface an actionable diagnostic instead of silently failing.
- **Vendor abstraction**: `trait SuperResProvider { fn capabilities(); fn enable(factors); fn disable(); }` with `NvidiaDsr` first; `AmdVsr` (ADLX) and `IntelScaling` (IGCL, best-effort) in Phase 4.

### 3.2 Display mode plane

- Topology model: adapters → targets → modes, built from `QueryDisplayConfig` + `EnumDisplaySettingsEx`, keyed by stable device path (survives cable swaps better than display numbers).
- Switching: `ChangeDisplaySettingsEx` with `CDS_UPDATEREGISTRY`; multi-monitor topologies via `SetDisplayConfig`.
- **Safety protocol** (non-negotiable invariant): every non-native switch starts a revert timer (default 10 s). Confirmation arrives via UI click, hotkey, or tray. Timer state persists to disk so even a crash/power-loss mid-switch restores native on next start. A global panic hotkey (default `Ctrl+Alt+Shift+R`) always forces native.

### 3.3 DPI compensation plane

- Windows exposes no public "set display scale" API. We use the reverse-engineered `DisplayConfigGetDeviceInfo(type=-3)` / `DisplayConfigSetDeviceInfo(type=-4)` protocol (proven by [lihas/windows-DPI-scaling-sample](https://github.com/lihas/windows-DPI-scaling-sample); applies live, no logoff).
- Compensation math: switching 1080p→2160p doubles logical density, so DPI goes 100%→200% (or user-tuned offset, e.g. 175% for "same size, slightly more real estate"). Per-profile override supported.
- Risk: undocumented ⇒ guarded by a version canary — validated read-back after every write; on mismatch we roll DPI back and disable the plane for the session.

## 4. Automation (crate: `conductor`)

Declarative rules, evaluated by a small deterministic engine (priority-ordered, first-match wins per trigger class):

```toml
[[rule]]
name     = "Photoshop gets DLDSR"
trigger  = { foreground = "Photoshop.exe" }
profile  = "dldsr-2.25x"
restore  = "on-exit"          # on-exit | on-background | manual

[[rule]]
trigger  = { power = "battery" }
profile  = "native"
priority = 100                 # overrides app rules
```

- **Watchers**: process start/stop via WMI event subscription (`Win32_ProcessStartTrace`), foreground changes via `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)`, power via `PowerSettingRegisterNotification`. All watchers are debounced (mode switches cost ~1 s; flapping is unacceptable).
- **Conflict model**: power/thermal rules > manual pin > app rules > schedule. The active decision and its cause are always inspectable in UI ("why is my resolution X right now?").
- Hotkeys: `RegisterHotKey`, user-remappable, with cycle-through-profiles action.

## 5. UI — Chamber (Tauri 2 + Svelte 5)

- **Tray-first**: the tray menu is a full remote control (profiles, pin, panic-revert); the window is the design showpiece.
- **Design language**: "standing waves" — the resonance motif. Dark-first, harmonic ring visualization of available factors (native at center, 1.78× / 2.25× / 4× as orbits), spring-physics micro-animations (Motion One), reduced-motion respected, fluid type scale. Light theme derived, not an afterthought.
- **Onboarding wizard**: GPU/driver/panel detection → capability card → guided first switch with live revert countdown → DPI calibration preview (side-by-side text sample).
- **IPC contract**: typed Tauri commands (`tuner/*`, `conductor/*`, `config/*`) + one event channel `state/changed` carrying the full serialized app state (UI is a pure function of that state; no imperative UI mutation from backend).
- All UI copy in English first; RU localization via Fluent (`.ftl`) in polish phase.

## 6. Data & config

- Config: TOML at `%APPDATA%/Resonance/config.toml` (schema versioned, `serde` + migration on load). Profiles and rules are plain files — Git-friendly, exportable, shareable.
- Logs: `tracing` → rotating file in `%LOCALAPPDATA%/Resonance/logs` + in-app diagnostics view. No telemetry, no network calls except the updater (opt-in).

## 7. Failure model & risk register

| Risk | Likelihood | Mitigation |
|---|---|---|
| DRS setting IDs shift across driver versions | Med | runtime enumeration + name match; ID table as fallback; canary validation |
| Undocumented DPI API breaks in a Windows update | Low-Med | read-back verification, plane auto-disable, native fallback |
| Display ends in unsupported mode (black screen) | Low | revert timer persisted to disk; panic hotkey; native-on-boot guard |
| DSR unavailable (HDR on, some hybrid-GPU laptops) | Med | capability probe up-front; honest "why unavailable" UI |
| Text softness at non-integer factors | Certain (inherent) | prefer DLDSR/integer factors in recommendations; smoothness slider with live preview |

## 8. Quality gates

- `cargo clippy -- -D warnings`, `rustfmt`, TS strict, `svelte-check` — enforced in CI (GitHub Actions) on every push.
- Tests only where they pay: rules-engine and config-migration unit tests (pure logic, high regression value). Hardware planes are verified by a scripted manual checklist (`docs/VERIFY.md`, Phase 1) — real display switching cannot be meaningfully unit-tested.
- Every phase ends with a runnable artifact (see [ROADMAP.md](ROADMAP.md)).
