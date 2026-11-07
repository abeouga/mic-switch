# Mic Switch

**English** | [日本語](README.ja.md)

A tiny Windows tray app that switches your default **microphone and speaker** in one click — no more digging through Sound settings every time you plug in a USB mic.

<p align="center"><img src="docs/screenshot.jpg" width="320" alt="Mic Switch screenshot"></p>

## Features

- **One-click switching** for input (microphones) and output (speakers / headphones), selected with a segmented control
- Switches **all roles at once** — regular apps *and* communication apps such as Discord, Teams and Zoom
- **Auto-switch**: star (★) a device and it becomes the default as soon as it is plugged in. Devices are matched by name, so changing USB ports still works
- Volume slider and mute button for the current device
- Lives in the **system tray**; optional start with Windows
- Frosted-glass UI using the Windows Acrylic backdrop
- English and Japanese UI (follows the Windows display language)
- Small native binary (~3 MB), built with Rust + [Tauri 2](https://tauri.app/)

## Requirements

- Windows 10 version 1809 or later / Windows 11 (x64)
- [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) (preinstalled on Windows 11)

## Install

### Option A: download (no build tools needed)

1. Download `MicSwitch.exe` from the [Releases](../../releases) page.
2. Put it anywhere you like and run it.

The exe is not code-signed, so Windows SmartScreen may warn you the first time. Click **More info → Run anyway**.

### Option B: build from source

```powershell
git clone <this repository>
cd mic-switch
.\setup.cmd
```

`setup.cmd` (or `setup.ps1`) does everything for the current user, without administrator rights:

1. Installs Rust via rustup if it is missing
2. Checks for the MSVC C++ build tools and offers to install them with `winget`
3. Builds the release binary
4. Installs `MicSwitch.exe` to `%LOCALAPPDATA%\Programs\MicSwitch` and adds Desktop / Start menu shortcuts

Options:

| Option | Description |
|---|---|
| `-InstallDir <path>` | Install somewhere else (e.g. `D:\Apps\MicSwitch`) |
| `-RustDir <path>` | Install Rust under this folder instead of `%USERPROFILE%` (handy when C: is short on space) |
| `-NoShortcut` | Skip creating shortcuts |
| `-Uninstall` | Remove the app, its shortcuts, its settings and the startup entry |

```powershell
.\setup.cmd -InstallDir D:\Apps\MicSwitch -RustDir D:\dev
```

## Usage

| Action | How |
|---|---|
| Switch input / output list | Segmented control at the top |
| Make a device the default | Click its card |
| Auto-switch when plugged in | Click ★ on the card (and keep **Auto-switch** on) |
| Mute / volume | Round button and slider at the top |
| Hide the window | × button — the app keeps running in the tray |
| Open again / quit | Left-click the tray icon / right-click → Quit |

## Where settings are stored

- If the folder containing `MicSwitch.exe` is writable, settings live next to it in `data\` (portable mode).
- Otherwise (e.g. under `Program Files`) they go to `%LOCALAPPDATA%\MicSwitch`.
- "Start with Windows" adds `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\MicSwitch`.

## How it works

Windows has no public API for changing the default audio device. Like other tools of this kind (and the Sound settings page itself), Mic Switch uses the undocumented `IPolicyConfig` COM interface. It has been stable since Windows 7, but a future Windows update could change it.

Device changes are detected by polling the device list every 1.5 seconds.

## Development

```powershell
cargo build --release        # output: target\release\mic-switch.exe
```

| Path | Contents |
|---|---|
| `src/audio.rs` | Core Audio wrapper (enumerate devices, set default, volume) |
| `src/main.rs` | Tauri app, tray icon, device watcher, commands |
| `src/config.rs` | Settings file and startup registration |
| `ui/` | Front-end (plain HTML / CSS / JS, no build step) |
| `tools/make-icon.ps1` | Regenerates `icons/` |

Pushing a tag such as `v0.1.0` builds the exe on GitHub Actions and attaches it to a release.

## License

[MIT](LICENSE)
