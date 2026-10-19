<p align="center">
  <img src="assets/icon.png" alt="Tunefast logo" width="88" height="88">
</p>

<h1 align="center">Tunefast</h1>

<p align="center"><strong>Performance modes, fans and system vitals for your Victus 16, native and fast.</strong><br>The hardware controls you open every day, in a 13 MB app that idles at about 22 MB of RAM.</p>

<p align="center">
  <a href="https://github.com/Readtt/tunefast/releases/latest"><strong>Download</strong></a> ·
  <a href="#build-from-source">Build from source</a> ·
  <a href="#how-it-works">How it works</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Built_with-Rust-dea584" alt="Built with Rust">
  <img src="https://img.shields.io/badge/Windows-24292f" alt="Windows">
  <img src="https://img.shields.io/badge/License-MIT-blue" alt="MIT license">
</p>

Tunefast is a from-scratch native rebuild, in Rust with [iced](https://iced.rs)
drawn on the CPU, of the hardware pages of the gaming hub app that ships on HP
Victus 16 (16-d1xxx) laptops: performance modes, fan speed, auto switch to ECO,
temperatures and system vitals. It keeps the interface you know and leaves out
the store, offers, game library, gallery and sign-in. It uses about 22 MB of
RAM instead of 536, and is ready in 0.7 s instead of 13.

![Tunefast, Performance Control](docs/screenshot.png)

## The numbers

<!-- bench:start -->
| Metric | Original app | Tunefast | Difference |
|---|---:|---:|---:|
| RAM (private working set, all processes) | 536 MB | 22 MB | 24.7× less |
| Committed memory (private bytes) | 811 MB | 24 MB | 33.5× less |
| Peak RAM while idle | 541 MB | 22 MB | 24.9× less |
| Time to first window | 1.06 s | 0.32 s | 3.3× less |
| Time until idle (ready) | 13.0 s | 0.68 s | 19.2× less |
| CPU while idle (% of one core) | 9.4% | 0.0% | n/a |
| Processes | 2 | 1 | 2.0× less |
| Threads | 201 | 25 | 8.0× less |
| Install size | 447 MB | 13 MB | 35.7× less |

Measured with `tools/bench.py` on Windows 11 (10.0.26200), 16 logical cores,
15.7 GB RAM, display scale 1.25, median of 5 runs each. RAM is the private
working set summed over every process of the app, sampled after it went idle.
The original was measured on its start page with its always-running
background process; Tunefast in live mode on Performance Control, reading its
sensors once a second.
<!-- bench:end -->

## How it feels

<!-- feel:start -->
| Interaction | Original app | Tunefast |
|---|---:|---:|
| Hover feedback on a page tab | 27.8 ms | not measured yet |
| Switching page (System Vitals / Performance Control) | 230.9 ms | not measured yet |
| Switching view (basic / advanced) | 192.7 ms | not measured yet |
| Scrolling the advanced view, first movement | 29.0 ms (38.4 pt per notch) | not measured yet |
| Fast scrolling, longest pause | 36 ms | not measured yet |

Time from the input to the first changed frame on screen, median of 15 trials
(`tools/feel.py`, 144 Hz display). The original's numbers are measured; the
rebuild's run moves the mouse, so it waits for a moment when the PC can be
left alone. Tunefast selects tabs, menu rows and modes on mouse-down, scrolls
the original's 48 units per wheel notch, and repaints only what changed.
<!-- feel:end -->

## What it does

| Feature | Status |
|---|---|
| Performance modes ECO, Quiet, Default, Performance, with the original's cards and descriptions | Supported (needs hardware control, below) |
| ECO's display side: 60 Hz on the laptop panel and a 60 fps NVIDIA cap, put back when you leave ECO | Supported |
| Fan Speed AUTO / MAX, disabled in ECO as in the original | Supported (needs hardware control) |
| Fan speed readout (RPM) | Supported (needs hardware control) |
| Auto Switch to Eco Mode on battery or a low-wattage adapter | Supported while Tunefast is open (needs hardware control) |
| CPU temperature, GPU temperature and CPU utilization, once a second while the window is open | Supported |
| System Vitals: GPU, CPU, RAM and storage tiles, your settings, network speed, top processes with STOP | Supported |
| Advanced view: per-core clocks and utilization, package temperature, GPU, RAM and network details with history charts | Supported |
| System information with COPY TO CLIPBOARD and PRINT | Supported |
| Temperature in °C or °F, colour states | Supported |
| Dark and light theme | Supported |
| Re-applies your mode and max fan after power events and resume | Supported (by the helper service) |
| Per-core CPU temperatures and CPU power in the advanced view | Not yet: the original reads them through its kernel driver; Tunefast uses no drivers |
| ECO's Intel Turbo Boost switch | Not yet: same reason (kernel driver) |
| Keyboard lighting | Not offered on this model by the original either |
| Network Booster, OMEN AI, optimizer, overlays | Not yet: software features, not laptop hardware |
| Store, deals, rewards, game library, gallery, sign-in, profiles | Left out on purpose |
| Overclocking, undervolting, custom fan curves | Left out: not offered on this model, and never done by Tunefast |

## Install

Download the zip from the
[latest release](https://github.com/Readtt/tunefast/releases/latest), unzip it
and run `tunefast.exe`. Sensors and system vitals work right away, without
admin rights.

**Hardware control** (modes, fan speed, fan RPM) needs one approval: open
**SETTINGS** in the left menu and choose **TURN ON HARDWARE CONTROL**. Windows
asks once to install `tunefast-helper.exe` (next to `tunefast.exe`) as a small
service. The Tunefast window itself always runs without admin rights. The
helper can make only the firmware calls the maker's app makes on this laptop,
with the same values: set the performance mode, set or read max fan, read the
fan speed, read the power adapter status and the system design data. Nothing
else can be sent through it. To remove it, run `tunefast-helper.exe uninstall`
from an elevated prompt.

To look around with sample data and no hardware access:

```sh
tunefast --demo
```

## Build from source

You need [Rust](https://rustup.rs) (the pinned version installs itself).
Hardware access is Windows-only; on macOS and Linux the app builds and runs
with its demo data (and, on Linux, needs `libxkbcommon-dev libwayland-dev`).

```sh
git clone https://github.com/Readtt/tunefast
cd tunefast
cargo run --release            # the real thing (Windows)
cargo run --release -- --demo  # sample data, no hardware access
```

## How it works

The interface is iced 0.14 on its CPU renderer (tiny-skia), laid out on the
original's 1624 × 940 design canvas and scaled to the window by the original's
own rule, so it looks the same at every size; it repaints only what changed.
Sensors come from Windows without admin rights: PDH counters for CPU load and
per-core clocks, the firmware's ACPI thermal zone for CPU temperature, NVML for
the NVIDIA GPU (asked only while the GPU is awake, so it is never woken up just
to be read), and they are read only for the page on screen and only while the
window is open. Firmware settings go through `tunefast-helper`, a Windows
service that understands a fixed list of requests over a local named pipe and
turns each into the exact BIOS WMI call (`hpqBIntM`) the maker's app sends on
this laptop. Choices show at once and are confirmed in the background; the last
session's numbers draw the first frame, and settings live in
`%APPDATA%\Tunefast\settings.json`.

The behaviour of the original was studied with
[REA](https://github.com/morluto/rea) (static analysis of the shipped app). No
original source code, artwork or fonts are included.

## Account and warranty notes

Tunefast touches no online service and needs no account. It sends only the
firmware commands and values that the maker's own app sends on this model, so
it adds no hardware risk beyond what that app does, but it is a third-party
tool: use it at your own risk. Its CPU temperature comes from the firmware's
thermal zone and can read a few degrees higher than the original's
driver-based package sensor.

## Not affiliated

Tunefast is an independent project. It is not affiliated with, endorsed by, or
connected to HP Inc. HP, OMEN and Victus are trademarks of HP Inc.; they are
named here only to say which laptop and app this works with.

## Acknowledgements

[iced](https://github.com/iced-rs/iced), the
[Montserrat](https://github.com/JulietaUla/Montserrat) and
[Zen Dots](https://fonts.google.com/specimen/Zen+Dots) typefaces (OFL),
[Lucide](https://lucide.dev) icons (ISC), [nvml-wrapper](https://github.com/Cldfire/nvml-wrapper),
[wmi-rs](https://github.com/ohadravid/wmi-rs), [windows-rs](https://github.com/microsoft/windows-rs)
and [REA](https://github.com/morluto/rea).

## License

[MIT](LICENSE)
