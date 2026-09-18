use std::process::ExitCode;
use std::time::Duration;

#[cfg(feature = "config")]
use attack_shark_r1::MouseConfig;
use attack_shark_r1::{
    AttackSharkR1, BatteryStatus, DriverError, UhidBatteryDevice, DEVICE_NAME,
    PRODUCT_ID_WIRED, PRODUCT_ID_WIRELESS, VENDOR_ID,
};

const CLI_FLAG_QUERY_CHARGE: &str = "-query-charge";
const CLI_FLAG_BATTERY: &str = "--battery";
const CLI_FLAG_BATTERY_SHORT: &str = "-b";
const CLI_FLAG_HELP: &str = "--help";
const CLI_FLAG_HELP_SHORT: &str = "-h";

const DEFAULT_POLL_INTERVAL_SECS: u64 = 5;
const STATE_CHARGING: &str = "charging";
const STATE_DISCHARGING: &str = "discharging";

fn is_battery_query_arg(arg: &str) -> bool {
    arg == CLI_FLAG_QUERY_CHARGE || arg == CLI_FLAG_BATTERY || arg == CLI_FLAG_BATTERY_SHORT
}

fn is_help_arg(arg: &str) -> bool {
    arg == CLI_FLAG_HELP || arg == CLI_FLAG_HELP_SHORT
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| is_battery_query_arg(a)) {
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

    if args.iter().any(|a| is_help_arg(a)) {
        println!("{DEVICE_NAME} daemon");
        println!("Reports battery and charging state to UPower, KDE Plasma, and Waybar.");
        println!();
        println!("Usage:");
        println!("  attack-shark-r1                # Run daemon (default)");
        println!("  attack-shark-r1 {CLI_FLAG_QUERY_CHARGE}  # Print battery percentage");
        println!("  attack-shark-r1 {CLI_FLAG_HELP}         # Show this help");
        return ExitCode::SUCCESS;
    }

    run_daemon(DEFAULT_POLL_INTERVAL_SECS)
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
    println!("Starting {DEVICE_NAME} daemon (interval: {interval_secs}s)...");

    let mut virtual_device: Option<UhidBatteryDevice> = None;
    let mut last_status: Option<BatteryStatus> = None;
    #[cfg(feature = "config")]
    let mut configured = false;

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

                            if changed {
                                let state_str = if status.is_charging {
                                    STATE_CHARGING
                                } else {
                                    STATE_DISCHARGING
                                };
                                println!("Battery: {}% [{state_str}]", status.percentage);

                                match virtual_device.as_mut() {
                                    Some(vdev) => {
                                        if let Err(e) =
                                            vdev.update_battery(status.percentage, status.is_charging)
                                        {
                                            eprintln!("Failed to send battery update to kernel: {e}");
                                        }
                                    }
                                    None => {
                                        let pid = if mouse.is_wired() {
                                            PRODUCT_ID_WIRED
                                        } else {
                                            PRODUCT_ID_WIRELESS
                                        };
                                        match UhidBatteryDevice::create(
                                            DEVICE_NAME,
                                            VENDOR_ID as u32,
                                            pid as u32,
                                            status.percentage,
                                            status.is_charging,
                                        ) {
                                            Ok(d) => {
                                                println!("Registered virtual power supply on /dev/uhid.");
                                                virtual_device = Some(d);
                                            }
                                            Err(e) => {
                                                eprintln!("Failed to create UHID virtual device: {e}");
                                                eprintln!("Make sure /dev/uhid is accessible (check udev rules or run as root).");
                                            }
                                        }
                                    }
                                }

                                last_status = Some(status);
                            }
                        }
                        Err(DriverError::Usb(rusb::Error::Timeout)) => {
                            // Mouse is asleep or stationary. Keep previous state.
                        }
                        Err(e) => {
                            println!("Mouse disconnected or USB error: {e}");
                            virtual_device = None;
                            last_status = None;
                            break;
                        }
                    }

                    std::thread::sleep(Duration::from_secs(interval_secs));
                }
            }
            Err(_) => {
                if virtual_device.is_some() {
                    println!("Mouse disconnected. Removing virtual power supply.");
                    virtual_device = None;
                    last_status = None;
                }
                #[cfg(feature = "config")]
                {
                    configured = false;
                }
            }
        }

        std::thread::sleep(Duration::from_secs(interval_secs));
    }
}

