# CPU Balancer

**English** | [Русский](README.ru.md)

![CPU Balancer — power plan editor](docs/screenshot.jpg)

Fine-tuning of a laptop CPU's power plans on Windows 10/11 — in a single small portable exe
with a Windows 11 style interface (Mica, light/dark theme, system accent colour).
Sliders, presets, rollback, and background apps kept on E-cores, all from the tray.

Built for Intel hybrid CPUs (Arrow Lake / Lunar Lake / Meteor Lake), but runs on any x86:
settings your system doesn't have are simply shown as unavailable. Interface in English and Russian.

## Getting started

1. Download `cpu-balancer.exe` from the [latest release](https://github.com/DonaK200097/cpu-balancer/releases/latest)
   and put it in any folder you can write to — its data (backups, settings) is kept next to it.
2. Run it and accept the administrator prompt (UAC) — Windows won't let you change power plans or app priorities without it.
3. Pick a power plan and start tuning.

Requires the WebView2 runtime (built into Windows 11; on Windows 10 it is usually installed with Edge).

## What you can tune

Every setting has separate values for "plugged in" and "on battery"; the column for the current power source is highlighted.

| Group | Settings |
|---|---|
| Main | EPP for P-cores and E-cores (speed ⟷ efficiency), turbo boost, max/min performance, P- and E-core frequency limits |
| Cores | core parking, how tasks are split between P/E-cores, keeping background work on E-cores (Windows 11 containment zones) |
| Other | responsiveness (latency hints), HWP vs. Windows control, PCIe power saving (ASPM), system cooling policy |

Each setting comes with a plain-language explanation and a hint about which direction runs cooler.
The ▼ marker on a slider shows the plan's original value. Click the number next to a slider to type an exact value.

Edits pile up and are applied only when you press **Save** (Ctrl+S). **Discard changes** drops them,
**Original** fills in the plan's original values.

## Presets

| Preset | Use it for |
|---|---|
| Quiet & cool | least noise and heat, background work on E-cores |
| Balance (Hydroc) | EPP 64/80, background on E-cores, no frequency spikes |
| Hydroc softer | same, but EPP halfway — more speed, slightly warmer |
| Performance | games and heavy workloads, snappy response |
| Max battery | changes "on battery" only: boost off, EPP 90/95 |

A preset only fills in values — you can fine-tune them before saving. Presets never touch frequency limits.

## Background apps on E-cores

The **Background apps** page lists running programs by current CPU load. For each one you can pick a mode:

| Mode | What it does |
|---|---|
| Normal | default Windows behaviour |
| Eco | EcoQoS — the same mechanism as "Efficiency mode" in Task Manager: Windows treats the app as background work, keeps it on E-cores at low clocks and doesn't wake the P-cores. Priority is lowered to Below normal |
| E-cores only | Eco + hard affinity to E/LP-E cores — the coolest option, but the app gets slower |

Good candidates: messengers, torrent clients, launchers, sync tools. Don't use it for games or apps you actively work in.

A mode lasts while the process is running. CPU Balancer in the tray re-applies it to every new process of the selected
apps right when it starts (within a fraction of a second) and re-checks every 30 s. To keep this working after a reboot,
turn on **Settings → Start at sign-in** (a Task Scheduler task that starts the app in the tray with admin rights, no UAC prompt).
The list is stored in `eco-apps.json` next to the app.

## Tray

Closing the window hides the app to the tray. The tray menu lets you open it, switch the power plan, pause auto-apply and exit.
While hidden, the app uses almost no CPU and never touches the discrete GPU.

## Rollback

The first time you open a plan, its original values are saved to `backups\<plan GUID>.json`.
The **Original** button brings them back at any time. Backups from version 1 are picked up automatically.

## G-Helper / Armoury Crate

These tools switch power plans together with their own modes (Silent / Balanced / Turbo),
so it's handy to tune each plan for its mode — CPU Balancer never switches plans by itself.
Power limits (PL1/PL2), fan curves and GPU clocks are still set in G-Helper / Armoury.

## Version 1 (PowerShell)

The original console version — no exe, pure PowerShell — lives in [`legacy/powershell`](legacy/powershell)
and in the [v1.0.0 release](https://github.com/DonaK200097/cpu-balancer/releases/tag/v1.0.0).

## Building from source

Rust (stable, MSVC), Node.js LTS, Visual Studio Build Tools (C++), then:

```
cargo install tauri-cli --version "^2" --locked
cd app && npm install
cd src-tauri && cargo tauri build --no-bundle
```

`cpu-balancer.exe --selftest report.txt` runs the release checks without the UI
(values vs. `powercfg`, write and restore, v1 backups, eco modes, the watcher).
