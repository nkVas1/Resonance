# ADR-0001: Driver-first super-resolution (not capture-based upscaling)

- **Status:** accepted · 2026-07-08
- **Context:** Two viable ways exist to show "more resolution than the panel has" on Windows:
  1. **Driver super-resolution** (NVIDIA DSR/DLDSR, AMD VSR): the driver exposes virtual modes above native; the OS genuinely renders at that resolution and the driver downsamples to the panel. System-wide by nature.
  2. **Capture-based upscaling** (Magpie, Lossless Scaling): capture a window, upscale it with shaders (FSR/Anime4K/…), present over it. Per-window by nature; adds capture latency; cursor/focus edge cases; and it *upscales* (low→native) rather than *supersamples* (above-native→native), so it cannot sharpen an already-native desktop.
- **Decision:** Resonance is built driver-first. The core product controls DSR/DLDSR (later VSR) plus display mode plus DPI as one orchestrated operation. Capture-based upscaling is a Phase 4 *complement* for unsupported hardware only, and may be delegated to Magpie rather than reimplemented.
- **Consequences:**
  - (+) True system-wide effect, zero runtime overhead beyond the GPU's own downsampling, no capture pipeline to maintain.
  - (+) Unique niche: no existing tool productizes DSR for the desktop with DPI compensation and automation.
  - (−) Depends on undocumented DRS setting IDs and an undocumented DPI API → mitigations in ARCHITECTURE §7 (runtime discovery, canaries, safe fallbacks).
  - (−) NVIDIA-only at first; vendor abstraction (`SuperResProvider`) keeps the door open.
