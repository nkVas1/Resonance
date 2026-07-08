# Phase 0 — Findings (2026-07-08)

Test rig: RTX 3060 Ti · driver 32.0.15.9636 (R575 line) · 1920×1080 **@180 Hz** panel · Windows 11 Home 26200.

## ✅ The core chain works end-to-end — today, with zero driver tweaks

```
$ drs-probe cycle 2880 1620 150 6
original: 1920x1080 @180Hz, scale 100%
switched: 2880x1620 @180Hz        # 2.25× supersampled desktop on a 1080p panel
scale: 150%                        # DPI compensation applied live
reverted to 1920x1080 @180Hz, scale 100%
```

**Key discovery:** the driver already exposes above-native GPU-scaled modes without any
DSR checkbox — `EnumDisplaySettingsExW` lists **2560×1440, 2880×1620, 3840×2160 at every
refresh rate up to 180 Hz** (exactly the 1.78× / 2.25× / 4.00× DSR ladder), and
`ChangeDisplaySettingsExW` switches to them instantly (<1 s). The GPU renders the full
virtual resolution and downsamples; the panel keeps receiving its native signal.
Consequence: **Resonance needs no driver mutation for its core flow** — mode switch +
DPI compensation is sufficient. Driver-side DSR/DLDSR unlock is now a *quality*
enhancement (better downsample filter), not a prerequisite.

## DSR - Factors is NOT a DRS setting on modern drivers

- `NvAPI_DRS_EnumAvailableSettingIds` exports **125** named settings on this driver — none
  DSR-related (full dump: [phase0-dump.txt](phase0-dump.txt)).
- The community-circulated ID `0x000F00BA` is **Resizable BAR**, not DSR factors.
- Independent confirmation: GameEnvSetter (GitHub) hit the same wall (`DSR设置（需要补充定义）`).
- NVCP manages DSR through a private driver interface; the driver persists it in registry:
  `HKLM\SYSTEM\CurrentControlSet\Services\nvlddmkm\State\DisplayDatabase\<display>\`
  - `SmoothScalingData` (32 B) — header describing the factor table (currently zeroed = off)
  - `SmoothScalingMultiplierData` (280 B = 10 × 28 B slots)
  - `UpScalingData` / `UpScalingMultiplierData` — likely the DL/quality-upscaling twin
- **Container format (new, cracked):** every value = records of
  `[version 0x1DB|0x2DB u32][record_size u32][payload][checksum u32]` where
  checksum = **byte-sum of all preceding record bytes** (verified on 5 value types).
- **Old payload format** (pre-container, documented by jim2point0's DSR calculator source):
  `[enabled u32][smoothness u32][count u32]` + 10 × `[mulX u16·2pad][mulY u16·2pad][4 zero]`,
  where mul = round(desired/native × 10000). New-format payload field mapping + DLDSR bit
  still unknown → obtain ground truth later by diffing registry around a one-time NVCP
  toggle (deferred to the DSR-quality plane, Phase 4).

## DPI plane — working, with one Win11 correction

- Undocumented `DisplayConfigGetDeviceInfo(type=-3)`: packet must be **exactly 32 bytes**
  (36+ → error 87). `min_rel`/`max_rel` are valid (0..3 = 100%..175% on this panel), but
  **`cur_rel` is garbage** on Win11 26200 when no explicit override exists (observed
  1234568). → Read current scale via documented `GetDpiForMonitor(MDT_EFFECTIVE_DPI)`
  (requires per-monitor-v2 DPI awareness), keep -3 only for min/max.
- `DisplayConfigSetDeviceInfo(type=-4)` **works live** (100→125→150→100 verified with
  read-back canary; propagation < 100 ms).

## NVAPI FFI notes (for `tuner`)

- QueryInterface IDs verified against nvidiaProfileInspector (primary + fallback pairs);
  all 13 functions resolved on this driver. Calls succeed with struct sizes
  `NVDRS_SETTING` = 12320 B, `NVDRS_SETTING_VALUES` = 414112 B (version = size | 1<<16).
- `NVAPI_SETTING_NOT_FOUND` = **-160**.
- Gotcha: `Box::new(mem::zeroed::<414KB>())` overflows the stack in debug builds —
  allocate via `alloc_zeroed`.

## Implications for the roadmap

1. Phase 1 `tuner` = mode switch + DPI + safety guard (all proven primitives, no admin).
2. DSR/DLDSR registry unlock moves to Phase 4 ("quality plane"): needs admin + display
   re-init, and NVCP-diff ground truth for the new blob format.
3. `doctor` should report: above-native modes present?, DPI min/max, panel native+Hz.
