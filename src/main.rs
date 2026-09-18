use std::process::ExitCode;
use std::time::Duration;

#[cfg(feature = "config")]
use attack_shark_r1::MouseConfig;
use attack_shark_r1::{AttackSharkR1, UhidBatteryDevice};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "-query-charge" || a == "--battery" || a == "-b") {
        match AttackSharkR1::open() {
            Ok(mut mouse) => match mouse.get_battery_percentage() {
                Ok(pct) => {
                    println!("{pct}");
                    return ExitCode::SUCCESS;
                }
                Err(e) => {
                    eprintln!("Error querying battery: {e}");
                    return ExitCode::FAILURE;
                }
            },
            Err(e) => {
                eprintln!("Error connecting to mouse: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Attack Shark R1 daemon");
        println!("Reports battery and charging state to UPower, KDE Plasma, and Waybar.");
        println!();
        println!("Usage:");
        println!("  attack-shark-r1                # Run daemon (default)");
        println!("  attack-shark-r1 -query-charge  # Print battery percentage");
        println!("  attack-shark-r1 --help         # Show this help");
        return ExitCode::SUCCESS;
    }

    run_daemon(5)
}

#[cfg(feature = "config")]
fn apply_config_if_present(mouse: &mut AttackSharkR1) {
    if let Some(path) = MouseConfig::find_config_file(None) {
        match MouseConfig::load_from_file(&path) {
            Ok(cfg) => {
                if let Err(e) = mouse.apply_config(&cfg) {
                    eprintln!("Warning: failed to apply config from {}: {e}", path.display());
                } else {
                    println!("Applied configuration from {}", path.display());
                }
            }
            Err(e) => {
                eprintln!("Warning: could not parse config {}: {e}", path.display());
            }
        }
    }
}

fn run_daemon(interval_secs: u64) -> ExitCode {
    println!("Starting Attack Shark R1 daemon (interval: {interval_secs}s)...");

    let mut virtual_device = match UhidBatteryDevice::create("Attack Shark R1", 0x1d57, 0xfa60) {
        Ok(d) => {
            println!("Registered virtual power supply on /dev/uhid.");
            d
        }
        Err(e) => {
            eprintln!("Failed to initialize UHID virtual device: {e}");
            eprintln!("Make sure /dev/uhid is accessible (check udev rules or run as root).");
            return ExitCode::FAILURE;
        }
    };

    println!("Daemon running. Monitoring mouse every {interval_secs}s.");

    let mut last_status: Option<attack_shark_r1::BatteryStatus> = None;
    #[cfg(feature = "config")]
    let mut configured = false;
    let mut initial = true;

    loop {
        match AttackSharkR1::open() {
            Ok(mut mouse) => {
                #[cfg(feature = "config")]
                if !configured {
                    apply_config_if_present(&mut mouse);
                    configured = true;
                }

                loop {
                    match mouse.get_battery_status() {
                        Ok(status) => {
                            let changed = match &last_status {
                                Some(prev) => {
                                    prev.percentage != status.percentage
                                        || prev.is_charging != status.is_charging
                                }
                                None => true,
                            };

                            if changed || initial {
                                let state_str = if status.is_charging {
                                    "charging"
                                } else {
                                    "discharging"
                                };
                                println!("Battery: {}% [{state_str}]", status.percentage);
                                if let Err(e) =
                                    virtual_device.update_battery(status.percentage, status.is_charging)
                                {
                                    eprintln!("Failed to send battery update to kernel: {e}");
                                }
                                last_status = Some(status);
                                initial = false;
                            }
                        }
                        Err(attack_shark_r1::DriverError::Usb(rusb::Error::Timeout)) => {
                            // Mouse is asleep or stationary. Keep previous state.
                        }
                        Err(attack_shark_r1::DriverError::Usb(rusb::Error::NoDevice)) => {
                            eprintln!("Mouse disconnected or receiver unplugged.");
                            break;
                        }
                        Err(e) => {
                            eprintln!("USB communication error: {e}");
                            break;
                        }
                    }

                    std::thread::sleep(Duration::from_secs(interval_secs));
                }
            }
            Err(_) => {
                #[cfg(feature = "config")]
                {
                    configured = false;
                }
            }
        }

        std::thread::sleep(Duration::from_secs(interval_secs));
    }
}
