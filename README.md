# Attack Shark R1 Linux driver

Linux driver and battery daemon for the Attack Shark R1 mouse. It supports both the 2.4 GHz wireless dongle and wired USB-C mode.

By default, the build produces a 323 KB daemon that monitors battery level and charging state. It feeds this data to the kernel via `/dev/uhid`, so UPower, KDE Plasma, and Waybar pick it up automatically without extra scripts. Configuration features like DPI and polling rate are optional Cargo flags.

## Features

- **Battery daemon (default).** Reads battery percentage and charging state every 5 seconds. Updates the kernel power supply interface through `/dev/uhid`.
- **Desktop integration.** Shows up in KDE Plasma system tray and Waybar through their standard UPower backends.
- **Direct query.** `attack-shark-r1 -query-charge` prints the battery number for shell scripts.
- **Hardware tuning (optional).** Compile with `--features tuning` to configure DPI stages, polling rate, sleep timers, and debouncing from an INI file.

## Build and install

### Requirements

- Rust 1.75 or newer
- `libusb-1.0`

### Build

```bash
# Default battery daemon (323 KB)
cargo build --release

# With hardware tuning features
cargo build --release --features tuning
```

### Install

```bash
# Installs binary to /usr/local/bin, udev rules, and systemd service
sudo make install

# Enable and start the daemon
sudo systemctl enable --now attack-shark-r1.service
```

## How UPower and KDE integration works

KDE Plasma and Waybar read mouse batteries through UPower (`org.freedesktop.UPower`), which watches `/sys/class/power_supply/`. UPower does not provide a D-Bus API for user processes to register a power supply directly, so this daemon uses Linux `/dev/uhid`.

1. The daemon creates a virtual HID device through `/dev/uhid` using standard HID Battery System usages (`AbsoluteStateOfCharge` and `Charging`).
2. The kernel hid-input subsystem creates `/sys/class/power_supply/hid-...-battery`.
3. UPower picks up the sysfs device and publishes it on D-Bus.
4. KDE Plasma displays the battery percentage and charging bolt icon in the system tray. Waybar displays it with its built-in `upower` module.
5. When the mouse is stationary on wireless, its radio sleeps to save battery. The daemon keeps the last known state active in UPower until the mouse moves again.

### Check UPower device

```bash
upower -e | grep -i hid
# /org/freedesktop/UPower/devices/battery_hid_0003o1D57oFA60x002B_battery_1

upower -i $(upower -e | grep -i hid)
```

### Waybar module

Add the native `upower` module to your Waybar configuration:

```jsonc
"upower": {
    "icon-size": 16,
    "hide-if-empty": false,
    "tooltip": true,
    "tooltip-spacing": 20,
    "show-icon": true
}
```

## CLI usage

```bash
# Run daemon in foreground
attack-shark-r1

# Print current battery percentage
attack-shark-r1 -query-charge
```

## Optional tuning features

If built with `--features tuning` (or individual flags `dpi`, `polling-rate`, `sleep`, `config`), the daemon can apply hardware settings on startup from `~/.config/attack-shark-r1.ini` or `/etc/attack-shark-r1.ini`:

```ini
polling_rate      = 1000
sleep_time        = 6.0
deep_sleep_time   = 12
key_response_time = 4

dpis = 800 1600 3200 4000 5000 12000
active_dpi = 3

ripple_control = false
angle_snap     = false
```

## Hardware register map

Hardware reports and registers are defined in [`attack_shark_r1.ddsl`](./attack_shark_r1.ddsl) using the [`device-driver`](https://device-driver.com/) framework:

- Endpoint `0x83`: Battery status (Report ID 3, 8 bytes). Contains device ID, status, flags for charging or discharging, and raw charge value.
- Feature Report `0x04`: 6-stage DPI profile and sensor flags (56 bytes).
- Feature Report `0x05`: Sleep delay, deep sleep, debounce timing, and checksum (15 bytes).
- Feature Report `0x06`: Polling rate (9 bytes).

## Using as a library

```rust
use attack_shark_r1::AttackSharkR1;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut mouse = AttackSharkR1::open()?;
    let status = mouse.get_battery_status()?;
    println!("Battery: {}% (charging: {})", status.percentage, status.is_charging);
    Ok(())
}
```

## License

MIT OR Apache-2.0
