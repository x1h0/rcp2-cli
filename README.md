# rcp2-cli

![License](https://img.shields.io/badge/license-MIT-blue)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![Platform](https://img.shields.io/badge/platform-linux-lightgrey)
![Status](https://img.shields.io/badge/status-experimental-red)
![Firmware](https://img.shields.io/badge/tested--with--firmware-1.7.3-blue)

Unofficial command-line tool and TUI for managing the RØDECaster Pro II via USB HID.

**This is experimental software. Use at your own risk.**
The protocol was reverse-engineered from USB captures, based on [JUCE ValueTreeSynchroniser](https://docs.juce.com/master/classValueTreeSynchroniser.html), and tested only with firmware 1.7.3. Other versions may not work, and there is no guarantee for correctness or safety.
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
- Start, pause, and stop recording
- Trigger pads
- Transfer mode: browse internal storage (eMMC) or SD card, download files and folders
- Monitor device property updates in real-time
- Dump full device state tree as JSON

Many device features are not accessible through this tool, including mixer/EQ/effects configuration, firmware updates, Bluetooth setup, and more.


## Requirements

- Linux (uses hidraw backend)
- RØDECaster Pro II connected via the main USB-C port
- Rust toolchain (stable) with `cargo` to build and install
- udev rules for non-root access (see below)
- The `usbhid` quirk to avoid the on-exit device freeze (see [Preventing the Device Freeze](#preventing-the-device-freeze-recommended))
- Build: `libudev-dev libwayland-dev` (Debian/Ubuntu) or `systemd-libs wayland` (Arch)
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
# Launch the TUI (trigger/edit pads, upload sounds, control recording, transfer)
rcp2-cli tui

# Skip the disclaimer screen
rcp2-cli --i-know-what-i-do tui

# Or via environment variable
RCP2_ACCEPT_RISK=1 rcp2-cli tui

# Dry-run: connect and read normally, but log writes instead of sending them.
# Transfer is disabled in dry-run. In the TUI the log never clutters the screen:
# it is buffered during the session and printed when the TUI exits, or written
# live if you redirect stderr:
rcp2-cli tui --dry-run 2> trace.log
```

Commands:

- `rcp2-cli connect [--full]` - show device info and firmware (serial hidden unless `--full`)
- `rcp2-cli dump [--full]` - dump state tree as JSON (sensitive values redacted unless `--full`)
- `rcp2-cli monitor` - stream property updates in real-time
- `rcp2-cli record status [--json]` - show recording status
- `rcp2-cli record interactive` - live recording control (start/pause/stop)
- `rcp2-cli transfer interactive [--storage <emmc|sd>]` - browse storage and download files
- `rcp2-cli fader list [--json]` - list faders with mute/listen state and level
- `rcp2-cli fader mute <N> [on|off|toggle]` - mute a fader (0-based index)
- `rcp2-cli fader listen <N> [on|off|toggle]` - toggle the Listen button for a fader
- `rcp2-cli pad trigger <BANK> <PAD> [--hold <MS>] [--no-restore]` - trigger a soundpad (0-based bank/pad; restores the previous bank unless `--no-restore`)
- `rcp2-cli pad bank [BANK] [--json]` - switch the device to a pad bank and leave it selected (0-based); omit BANK to print the current bank

Global flags: `-v`/`-vv` for verbosity and `--dry-run` (log writes instead of sending them).

Press `?` inside the TUI to see all available hotkeys.


## Preventing the Device Freeze (recommended)

The firmware sends its updates with a blocking write and no timeout, and Linux stops
reading the device once the last program closes its handle. The device then blocks on
its next update and freezes until you replug the cable. The kernel's
`HID_QUIRK_ALWAYS_POLL` quirk keeps the device drained at all times and avoids this.

Add it to your kernel command line and reboot, with your product ID (from `lsusb -d 19f7:`)
in place of `0x0037`:

```
usbhid.quirks=0x19f7:0x0037:0x00000400
```

With GRUB, append it to `GRUB_CMDLINE_LINUX_DEFAULT` in `/etc/default/grub`, then run
`sudo grub-mkconfig -o /boot/grub/grub.cfg`.


## License

This project is licensed under the [MIT License](LICENSE).
