# Attack Shark R1 Linux Driver (Rust)

High-performance driver and CLI utility for the **Attack Shark R1** mouse (both 2.4GHz wireless dongle and wired USB-C mode) written **100% in Rust**, modeled with formal hardware **DDSL** specifications using the [**device-driver**](https://device-driver.com/) toolkit and direct USB communication via [`rusb`](https://crates.io/crates/rusb).

---

## ⚡ Features

- **Battery Querying:**
  - Full drop-in compatibility with legacy `-query-charge` (prints raw number, e.g., `90`).
  - Modern `--battery` / `-b`: battery charge percentage.
  - `--json`: structured output for status bar integration (**Waybar**, **Polybar**, **i3blocks**).
  - `--status` / `-s`: human-readable report with connection and battery diagnostics.
- **Polling Rate Configuration:** Supports 125Hz, 250Hz, 500Hz, and 1000Hz.
- **DPI Configuration:**
  - 6 stages fully customizable from 100 to 18000 DPI (in increments of 100).
  - Active DPI stage selection (1 to 6).
  - Exact precomputed 180-entry hardware lookup table for flawless precision.
- **Power Management & Latency:**
  - `sleep-time`: sleep delay in seconds (0.5s to 30.0s).
  - `deep-sleep-time`: deep sleep delay in minutes (1m to 60m).
  - `key-response-time`: debounce / key response time in milliseconds (even numbers, 4ms to 50ms).
- **Sensor Enhancements:**
  - `angle-snap`: angle snapping (true/false).
  - `ripple-control`: tracking ripple control (true/false).
- **INI Configuration File:** Supports loading and reapplying configuration files (`~/.config/attack-shark-r1.ini` or `/etc/attack-shark-r1.ini`).
- **Safe & Non-Intrusive:** Interacts exclusively with vendor-specific `Interface 2`, never interrupting normal pointer events on `Interface 0`.

---

## 🛠️ Architecture & DDSL Hardware Specification

This driver is designed around the [**device-driver**](https://device-driver.com/) framework as documented in the [device-driver book](https://device-driver.com/book/). Hardware registers and USB reports are formally specified in [`attack_shark_r1.ddsl`](./attack_shark_r1.ddsl):

```ddsl
device AttackSharkR1Device {
    register-address-type: u8,
    default-access: RW,
    default-byte-order: LE,

    /// Battery status report received on endpoint 0x83
    register BatteryReport {
        address: 0x03,
        access: RO,
        fields: fieldset _ {
            size-bytes: 5,
            field report_id 7:0 -> uint,
            field header 15:8 -> uint,
            field status 23:16 -> uint,
            field flags 31:24 -> uint,
            field charge_raw 39:32 -> uint,
        },
    },

    /// Polling rate configuration (Feature Report 0x06)
    register PollingRateConfig {
        address: 0x06,
        access: WO,
        fields: fieldset _ {
            size-bytes: 9,
            field report_id 7:0 -> uint,
            field command 15:8 -> uint,
            field sub 23:16 -> uint,
            field rate_raw 39:24 -> uint,
        },
    },
    ...
}
```

The specification is compiled into zero-cost, type-safe Rust code by `device_driver::compile!`, providing register operations that guarantee proper bit packing and endianness at compile time.

---

## 🚀 Build & Installation

### Requirements
- Rust 1.75+ (Cargo)
- `libusb-1.0` (installed by default on most Linux distributions)

### Compilation
```bash
cd ~/projects/attack-shark-r1
cargo build --release
```

The optimized binary is built at `target/release/attack-shark-r1`.

### System-wide Installation
```bash
sudo make install
```
This installs the binary to `/usr/local/bin/attack-shark-r1` and installs the udev rules into `/etc/udev/rules.d/99-attack-shark-r1.rules` so unprivileged users can access the mouse without `sudo`.

---

## 📖 CLI Usage Examples

### 1. Query Battery Percentage
```bash
# Legacy syntax (backward-compatible with original driver):
attack-shark-r1 -query-charge
# 90

# Standard flag:
attack-shark-r1 --battery
# 90
```

### 2. Full Device Status
```bash
attack-shark-r1 --status
```
Output:
```text
=== Attack Shark R1 Status ===
Connection: Wireless (2.4G Receiver)
Battery:    90%
Raw Charge: 9/10
Status byte:0x40
```

### 3. JSON Output (Waybar / Polybar Integration)
```bash
attack-shark-r1 --json
```
Output:
```json
{"battery":90,"class":"normal","is_wired":false,"percentage":90,"raw_charge":9,"status_code":64,"tooltip":"Attack Shark R1: 90%"}
```

#### Waybar Configuration Example (`~/.config/waybar/config`):
```json
"custom/mouse-battery": {
    "format": "󰍽 {}%",
    "interval": 60,
    "exec": "attack-shark-r1 --json",
    "return-type": "json",
    "tooltip": true
}
```

### 4. Change Polling Rate
```bash
attack-shark-r1 -p 1000
```

### 5. Configure DPI Stages
```bash
# Set active stage to 2:
attack-shark-r1 --active-dpi 2

# Customize stage 1 to 800 DPI and stage 2 to 1600 DPI:
attack-shark-r1 --dpi 1=800 --dpi 2=1600
```

### 6. Apply INI Configuration File
```bash
# Generate default configuration:
attack-shark-r1 --generate-config > ~/.config/attack-shark-r1.ini

# Reapply configuration to the mouse:
attack-shark-r1 --reapply-config
```

---

## 📄 Udev Rules

To interact with the mouse without root privileges, `99-attack-shark-r1.rules` provides:
```udev
SUBSYSTEM=="usb", ATTR{idVendor}=="1d57", ATTR{idProduct}=="fa60", MODE="0666"
SUBSYSTEM=="usb", ATTR{idVendor}=="1d57", ATTR{idProduct}=="fa61", MODE="0666"
```

---

## 📦 Using as a Rust Library (`lib.rs`)

You can also use this crate as a library in other Rust projects:

```rust
use attack_shark_r1::{AttackSharkR1, PollingRate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut mouse = AttackSharkR1::open()?;
    
    // Read battery percentage
    let battery = mouse.get_battery_percentage()?;
    println!("Battery: {battery}%");

    // Set polling rate to 1000Hz
    mouse.set_polling_rate(PollingRate::Hz1000)?;

    Ok(())
}
```

---

## ⚖️ License

MIT OR Apache-2.0.
