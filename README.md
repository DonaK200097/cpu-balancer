# CPU Balancer

**English** | [Русский](README.ru.md)

![CPU Balancer — power plan editor](docs/screenshot.webp)

A console app for fine-tuning a laptop CPU's power plans on Windows 10/11.
Sliders, toggles, presets and rollback — no installation, no dependencies, pure PowerShell.

Built for Intel hybrid CPUs (Arrow Lake / Lunar Lake / Meteor Lake), but runs on any x86:
settings your system doesn't have are simply shown as unavailable.

> The app interface is in Russian for now.

## Getting started

1. Download the archive from the [latest release](https://github.com/DonaK200097/cpu-balancer/releases/latest) and unzip it (or put `CPU-Balancer.cmd` and `CPU-Balancer.ps1` in the same folder).
2. Double-click `CPU-Balancer.cmd` and accept the administrator prompt (UAC) — Windows won't let you change power plans without it.
3. Pick a power plan and start tuning.

## What you can tune

Every setting has separate values for "plugged in" and "on battery".

| Group | Settings |
|---|---|
| Main | EPP for P-cores and E-cores (speed ⟷ efficiency), turbo boost, max/min performance, P- and E-core frequency limits |
| Cores | core parking, how tasks are split between P/E-cores, keeping background work on E-cores (Windows 11 containment zones) |
| Other | responsiveness (latency hints), HWP vs. Windows control, PCIe power saving (ASPM), system cooling policy |

Each setting comes with a plain-language explanation and a hint about which direction runs cooler.
The `│` marker on a slider shows the plan's original value.

## Presets

| # | Preset | Use it for |
|---|---|---|
| 1 | Quiet & cool | least noise and heat, background work on E-cores |
| 2 | Balance (Hydroc) | EPP 64/80, background on E-cores, no frequency spikes |
| 3 | Hydroc softer | same, but EPP halfway — more speed, slightly warmer |
| 4 | Performance | games and heavy workloads, snappy response |
| 5 | Max battery | changes "on battery" only: boost off, EPP 90/95 |

A preset only fills in values — you can fine-tune them before saving.

## Controls

- **Mouse:** drag a slider or click the scale, scroll wheel = one step (hold Shift for fine steps), click `◄` / the option name to switch, buttons and presets are clickable.
- **Keyboard:** ↑↓ select, ←→ change, Shift+←→ fine, PgUp/PgDn coarse, Tab — plugged in / battery, 1–5 — presets,
  S — save, Z — discard edits, R — original values, A — make the plan active, C — copy to the other column, Esc — back.

Edits pile up (marked with `*`) and are applied only when you press Save.

## Rollback

The first time you open a plan, its original values are saved to `backups\<plan GUID>.json`.
The "Original" button (R) brings them back at any time.

## G-Helper / Armoury Crate

These tools switch power plans together with their own modes (Silent / Balanced / Turbo),
so it's handy to tune each plan for its mode. Power limits (PL1/PL2) and fan curves
are still set in G-Helper / Armoury.
