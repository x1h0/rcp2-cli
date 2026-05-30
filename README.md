# rcp2-cli

![License](https://img.shields.io/badge/license-MIT-blue)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![Platform](https://img.shields.io/badge/platform-linux-lightgrey)
![Status](https://img.shields.io/badge/status-experimental-red)
![Firmware](https://img.shields.io/badge/tested--with--firmware-1.7.3-blue)

Unofficial command-line tool and TUI for managing the RØDECaster Pro II via USB HID.

**This is experimental software. Use at your own risk.**
The protocol was reverse-engineered and there is no guarantee for correctness or safety.
Using this tool may freeze your device, require a USB replug to recover, or corrupt your configuration and sounds.

<p align="center">
  <img src="demo.gif" alt="TUI demo" width="800">
</p>


## Features

- Live TUI showing faders, pots, recording state, SD card usage, and pad banks
- Browse and switch between pad banks (up to 8)
- View pad properties for all pad types (Sound, Effect, Special)
- Edit pad properties: name, color, gain (all types), play mode, loop, replay (Sound pads only)
- Upload, download, and replace sounds on Sound pads (WAV/MP3)
- Start, pause, and stop recording from the TUI
- Transfer mode: browse internal storage (eMMC) or SD card, download files and folders
- Monitor device property updates in real-time
- Dump full device state tree as JSON

Many device features are not accessible through this tool, including mixer/EQ/effects configuration, firmware updates, Bluetooth setup, and more.


## Requirements

- Linux (uses hidraw backend)
- RØDECaster Pro II connected via the main USB-C port
- udev rules for non-root access (see below)
- Build: `libudev-dev` (Debian/Ubuntu) or `systemd-libs` (Arch) for hidapi compilation
- Runtime: `lsblk`, `udisksctl` (for transfer mode mount detection)


## Installation

Directly from GitHub (no clone needed):

```sh
cargo install --git https://github.com/x1h0/rcp2-cli.git rcp2-cli
```

Or clone and install locally:

```sh
git clone https://github.com/x1h0/rcp2-cli.git
cd rcp2-cli
cargo install --path crates/rcp2-cli
```


## udev Rules

Copy the udev rules to allow non-root access to the device:

```sh
sudo cp udev/50-rodecaster.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```


## Usage

```sh
# Read-only mode (safe, but device buttons may freeze on exit)
rcp2-cli tui

# With send capabilities (edit pads, upload sounds, control recording)
rcp2-cli tui --allow-send

# Skip the disclaimer screen
rcp2-cli --i-know-what-i-do tui --allow-send

# Or via environment variable
RCP2_ACCEPT_RISK=1 RCP2_ALLOW_SEND=1 rcp2-cli tui
```

Other commands:

- `rcp2-cli connect` - show device info
- `rcp2-cli dump` - dump full state tree as JSON
- `rcp2-cli monitor` - stream property updates in real-time

Press `?` inside the TUI to see all available hotkeys.


## Known Issues

- After closing the app, device buttons may freeze until the USB cable is replugged. This appears to be a Linux-specific issue with how the HID connection is closed.
- The protocol is based on [JUCE ValueTreeSynchroniser](https://docs.juce.com/master/classValueTreeSynchroniser.html) and was reverse-engineered from USB captures. Tested with firmware 1.7.3. Other versions may not work correctly.


## License

This project is licensed under the [MIT License](LICENSE).
