use std::path::PathBuf;
use std::process::ExitCode;

use usage::Cli;

use attack_shark_r1::{
    AttackSharkR1, MouseConfig, PollingRate,
};

/// High-performance typed Linux driver and CLI for the Attack Shark R1 wireless and wired mouse.
/// Powered by Rust and device-driver.
#[derive(Cli, Debug)]
#[usage(bin = "attack-shark-r1", version = "0.1.0")]
struct Args {
    /// Query and output current battery charge percentage (e.g. 90)
    #[usage(short = 'b', long, alias = "query-charge")]
    battery: bool,

    /// Output detailed status in JSON format (useful for Waybar, Polybar, etc.)
    #[usage(long)]
    json: bool,

    /// Print comprehensive human-readable device status
    #[usage(short = 's', long)]
    status: bool,

    /// Set polling rate in Hz (125, 250, 500, 1000)
    #[usage(short = 'p', long = "polling-rate")]
    polling_rate: Option<u32>,

    /// Set active DPI stage (1-6)
    #[usage(long = "active-dpi")]
    active_dpi: Option<u8>,

    /// Set DPI stage value in format 'stage=dpi' (e.g. '1=800' or '3=3200')
    #[usage(long)]
    dpi: Vec<String>,

    /// Set sleep time in seconds [0.5, 30.0]
    #[usage(long = "sleep-time")]
    sleep_time: Option<f64>,

    /// Set deep sleep time in minutes [1, 60]
    #[usage(long = "deep-sleep-time")]
    deep_sleep_time: Option<u8>,

    /// Set key response / debounce time in ms [4, 50] (even numbers only)
    #[usage(long = "key-response-time")]
    key_response_time: Option<u8>,

    /// Enable or disable angle snapping (true|false)
    #[usage(long = "angle-snap")]
    angle_snap: Option<bool>,

    /// Enable or disable ripple control (true|false)
    #[usage(long = "ripple-control")]
    ripple_control: Option<bool>,

    /// Custom path to config file (INI)
    #[usage(short = 'c', long = "config")]
    config_path: Option<PathBuf>,

    /// Run as a background daemon creating a virtual power_supply via /dev/uhid for UPower & KDE Plasma
    #[usage(long)]
    daemon: bool,

    /// Daemon polling interval in seconds (default: 30)
    #[usage(long, default = "30")]
    interval: u64,

    /// Reapply all settings from configuration file
    #[usage(long = "reapply-config")]
    reapply_config: bool,

    /// Generate default configuration file to stdout
    #[usage(long = "generate-config")]
    generate_config: bool,
}

fn main() -> ExitCode {
    // Intercept legacy single-dash -query-charge or normalize args
    let raw_args: Vec<std::ffi::OsString> = std::env::args_os()
        .map(|arg| {
            if arg == "-query-charge" {
                std::ffi::OsString::from("--battery")
            } else {
                arg
            }
        })
        .collect();

    let argv_refs: Vec<&std::ffi::OsStr> = raw_args.iter().map(|s| s.as_os_str()).collect();

    let cli = match Args::parse_from_argv(&argv_refs) {
        Ok(c) => c,
        Err(usage::Error::Version { .. }) => {
            println!("attack-shark-r1 0.1.0");
            return ExitCode::SUCCESS;
        }
        Err(usage::Error::Help { cmd, long }) => {
            if let Some(help) = Args::render_help(cmd, long) {
                print!("{help}");
            }
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            let msg = Args::render_failure(&argv_refs, &e);
            eprint!("{msg}");
            return ExitCode::FAILURE;
        }
    };

    if cli.generate_config {
        let cfg = MouseConfig::default();
        print!(
            r#"# Attack Shark R1 Configuration File
polling_rate      = {}

sleep_time        = {}
deep_sleep_time   = {}
key_response_time = {}

dpis = {}
# selected dpi 1-6
active_dpi = {}

ripple_control = {}
angle_snap     = {}
"#,
            cfg.polling_rate.as_hz(),
            cfg.sleep_time,
            cfg.deep_sleep_time,
            cfg.key_response_time,
            cfg.dpis
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join(" "),
            cfg.active_dpi,
            cfg.ripple_control,
            cfg.angle_snap,
        );
        return ExitCode::SUCCESS;
    }

    if cli.daemon {
        return run_daemon(cli.interval);
    }

    // If no action flags are provided, show usage
    let has_action = cli.battery
        || cli.json
        || cli.status
        || cli.polling_rate.is_some()
        || cli.active_dpi.is_some()
        || !cli.dpi.is_empty()
        || cli.sleep_time.is_some()
        || cli.deep_sleep_time.is_some()
        || cli.key_response_time.is_some()
        || cli.angle_snap.is_some()
        || cli.ripple_control.is_some()
        || cli.reapply_config;

    if !has_action {
        eprintln!("Attack Shark R1 driver v0.1.0");
        eprintln!("Usage: attack-shark-r1 [OPTIONS]");
        eprintln!("Try 'attack-shark-r1 --help' for full options list.");
        eprintln!();
        eprintln!("Quick examples:");
        eprintln!("  attack-shark-r1 -query-charge       # Output battery percentage");
        eprintln!("  attack-shark-r1 --json              # Output JSON for Waybar / Polybar");
        eprintln!("  attack-shark-r1 --status            # Human readable status");
        eprintln!("  attack-shark-r1 -p 1000             # Set polling rate to 1000Hz");
        return ExitCode::SUCCESS;
    }

    // Try to open mouse
    let mut mouse = match AttackSharkR1::open() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error connecting to mouse: {e}");
            return ExitCode::FAILURE;
        }
    };

    // 1. Reapply config if requested
    if cli.reapply_config {
        let config_file = MouseConfig::find_config_file(cli.config_path.as_deref().and_then(|p| p.to_str()));
        match config_file {
            Some(path) => match MouseConfig::load_from_file(&path) {
                Ok(cfg) => {
                    if let Err(e) = mouse.apply_config(&cfg) {
                        eprintln!("Failed to apply config from {}: {e}", path.display());
                        return ExitCode::FAILURE;
                    }
                    println!("Applied configuration from {}", path.display());
                }
                Err(e) => {
                    eprintln!("Failed to load config from {}: {e}", path.display());
                    return ExitCode::FAILURE;
                }
            },
            None => {
                eprintln!("No configuration file found in search paths (~/.config/attack-shark-r1.ini or /etc/attack-shark-r1.ini)");
                return ExitCode::FAILURE;
            }
        }
    }

    // 2. Adjust individual settings if passed
    if let Some(rate_hz) = cli.polling_rate {
        let rate = match PollingRate::try_from(rate_hz) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Invalid polling rate: {e}");
                return ExitCode::FAILURE;
            }
        };
        if let Err(e) = mouse.set_polling_rate(rate) {
            eprintln!("Failed to set polling rate: {e}");
            return ExitCode::FAILURE;
        }
        println!("Polling rate set to {rate}");
    }

    if cli.sleep_time.is_some() || cli.deep_sleep_time.is_some() || cli.key_response_time.is_some() {
        let mut base_cfg = MouseConfig::default();
        if let Some(p) = MouseConfig::find_config_file(cli.config_path.as_deref().and_then(|p| p.to_str())) {
            if let Ok(c) = MouseConfig::load_from_file(&p) {
                base_cfg = c;
            }
        }
        if let Some(st) = cli.sleep_time {
            base_cfg.sleep_time = st;
        }
        if let Some(dst) = cli.deep_sleep_time {
            base_cfg.deep_sleep_time = dst;
        }
        if let Some(krt) = cli.key_response_time {
            base_cfg.key_response_time = krt;
        }
        if let Err(e) = mouse.set_sleep_times(
            base_cfg.sleep_time,
            base_cfg.deep_sleep_time,
            base_cfg.key_response_time,
        ) {
            eprintln!("Failed to set sleep times: {e}");
            return ExitCode::FAILURE;
        }
        println!(
            "Sleep times updated: sleep={:.1}s, deep_sleep={}m, debounce={}ms",
            base_cfg.sleep_time, base_cfg.deep_sleep_time, base_cfg.key_response_time
        );
    }

    if cli.active_dpi.is_some()
        || !cli.dpi.is_empty()
        || cli.angle_snap.is_some()
        || cli.ripple_control.is_some()
    {
        let mut base_cfg = MouseConfig::default();
        if let Some(p) = MouseConfig::find_config_file(cli.config_path.as_deref().and_then(|p| p.to_str())) {
            if let Ok(c) = MouseConfig::load_from_file(&p) {
                base_cfg = c;
            }
        }
        if let Some(ad) = cli.active_dpi {
            if !(1..=6).contains(&ad) {
                eprintln!("Invalid DPI stage '{ad}', must be 1..=6");
                return ExitCode::FAILURE;
            }
            base_cfg.active_dpi = ad;
        }
        if let Some(asnap) = cli.angle_snap {
            base_cfg.angle_snap = asnap;
        }
        if let Some(rc) = cli.ripple_control {
            base_cfg.ripple_control = rc;
        }
        for item in &cli.dpi {
            let parts: Vec<&str> = item.split('=').collect();
            if parts.len() != 2 {
                eprintln!("Invalid DPI specification '{item}', expected format 'STAGE=DPI' (e.g. '1=800')");
                return ExitCode::FAILURE;
            }
            let stage: usize = match parts[0].trim().parse() {
                Ok(s) if (1..=6).contains(&s) => s,
                _ => {
                    eprintln!("Invalid DPI stage '{}', must be 1..=6", parts[0]);
                    return ExitCode::FAILURE;
                }
            };
            let dpi_val: u32 = match parts[1].trim().parse() {
                Ok(d) => d,
                Err(_) => {
                    eprintln!("Invalid DPI value '{}'", parts[1]);
                    return ExitCode::FAILURE;
                }
            };
            base_cfg.dpis[stage - 1] = dpi_val;
        }

        if let Err(e) = mouse.set_dpi_profile(
            base_cfg.dpis,
            base_cfg.active_dpi,
            base_cfg.angle_snap,
            base_cfg.ripple_control,
        ) {
            eprintln!("Failed to set DPI profile: {e}");
            return ExitCode::FAILURE;
        }
        println!(
            "DPI profile updated: active stage={}, dpis={:?}, angle_snap={}, ripple_control={}",
            base_cfg.active_dpi, base_cfg.dpis, base_cfg.angle_snap, base_cfg.ripple_control
        );
    }

    // 3. Query battery / status
    if cli.battery {
        match mouse.get_battery_percentage() {
            Ok(pct) => {
                println!("{pct}");
            }
            Err(e) => {
                eprintln!("Error querying battery: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else if cli.json {
        match mouse.get_battery_status() {
            Ok(status) => {
                let class_name = if status.percentage <= 20 {
                    "critical"
                } else if status.percentage <= 40 {
                    "warning"
                } else {
                    "normal"
                };
                let wired_suffix = if status.is_wired { " (Wired)" } else { "" };
                println!(
                    r#"{{"battery":{},"percentage":{},"is_wired":{},"raw_charge":{},"status_code":{},"tooltip":"Attack Shark R1: {}%{}","class":"{}"}}"#,
                    status.percentage,
                    status.percentage,
                    status.is_wired,
                    status.raw_charge,
                    status.status_code,
                    status.percentage,
                    wired_suffix,
                    class_name
                );
            }
            Err(e) => {
                eprintln!("Error querying battery: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else if cli.status {
        match mouse.get_battery_status() {
            Ok(status) => {
                println!("=== Attack Shark R1 Status ===");
                println!("Connection: {}", if status.is_wired { "Wired (USB Cable)" } else { "Wireless (2.4G Receiver)" });
                println!("Battery:    {}%", status.percentage);
                println!("Raw Charge: {}/10", status.raw_charge);
                println!("Status byte:0x{:02x}", status.status_code);
            }
            Err(e) => {
                eprintln!("Error querying battery: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
}

fn run_daemon(interval_secs: u64) -> ExitCode {
    use attack_shark_r1::UhidBatteryDevice;

    println!("Starting Attack Shark R1 UPower / KDE battery daemon (interval: {interval_secs}s)...");

    let mut virtual_device = match UhidBatteryDevice::create("Attack Shark R1", 0x1d57, 0xfa60) {
        Ok(d) => {
            println!("Registered virtual power supply device in Linux kernel via /dev/uhid!");
            d
        }
        Err(e) => {
            eprintln!("Failed to initialize UHID virtual device: {e}");
            eprintln!();
            eprintln!("Note on permissions:");
            eprintln!("  1. Run with sudo: 'sudo attack-shark-r1 --daemon'");
            eprintln!("  2. Or enable the systemd service: 'sudo systemctl enable --now attack-shark-r1'");
            eprintln!("  3. Or add KERNEL==\"uhid\", TAG+=\"uaccess\", MODE=\"0666\" to /etc/udev/rules.d/99-attack-shark-r1.rules");
            return ExitCode::FAILURE;
        }
    };

    println!("Daemon active. Linux kernel, UPower, and KDE Plasma will now reflect the mouse battery.");

    let mut last_percentage: Option<u8> = None;

    loop {
        match AttackSharkR1::open() {
            Ok(mut mouse) => match mouse.get_battery_percentage() {
                Ok(pct) => {
                    if last_percentage != Some(pct) {
                        println!("Battery updated: {pct}% (notifying UPower / KDE Plasma)");
                        if let Err(e) = virtual_device.update_battery(pct) {
                            eprintln!("Failed to send battery update to kernel: {e}");
                        }
                        last_percentage = Some(pct);
                    }
                }
                Err(e) => {
                    eprintln!("Warning: could not read battery: {e}");
                }
            },
            Err(_) => {
                // Mouse is asleep or receiver unplugged
                last_percentage = None;
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(interval_secs));
    }
}

