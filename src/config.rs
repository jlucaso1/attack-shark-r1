use std::path::{Path, PathBuf};
use std::str::FromStr;

use ini::Ini;
use serde::{Deserialize, Serialize};

use crate::error::DriverError;
use crate::protocol::PollingRate;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MouseConfig {
    pub polling_rate: PollingRate,
    pub sleep_time: f64,
    pub deep_sleep_time: u8,
    pub key_response_time: u8,
    pub dpis: [u32; 6],
    pub active_dpi: u8,
    pub ripple_control: bool,
    pub angle_snap: bool,
}

impl Default for MouseConfig {
    fn default() -> Self {
        Self {
            polling_rate: PollingRate::Hz1000,
            sleep_time: 6.0,
            deep_sleep_time: 12,
            key_response_time: 4,
            dpis: [800, 1600, 3200, 4000, 5000, 12000],
            active_dpi: 3,
            ripple_control: false,
            angle_snap: false,
        }
    }
}

impl MouseConfig {
    pub fn validate(&self) -> Result<(), DriverError> {
        if self.sleep_time < 0.5 || self.sleep_time > 30.0 {
            return Err(DriverError::InvalidSleepTime(self.sleep_time));
        }
        if self.deep_sleep_time < 1 || self.deep_sleep_time > 60 {
            return Err(DriverError::InvalidDeepSleepTime(self.deep_sleep_time));
        }
        if self.key_response_time < 4 || self.key_response_time > 50 || self.key_response_time % 2 != 0 {
            return Err(DriverError::InvalidKeyResponseTime(self.key_response_time));
        }
        if self.active_dpi < 1 || self.active_dpi > 6 {
            return Err(DriverError::InvalidDpiStage(self.active_dpi));
        }
        for &dpi in &self.dpis {
            if !(100..=18000).contains(&dpi) || dpi % 100 != 0 {
                return Err(DriverError::InvalidDpi(dpi));
            }
        }
        Ok(())
    }

    /// Finds the configuration file from custom path or standard search locations.
    pub fn find_config_file(custom_path: Option<&str>) -> Option<PathBuf> {
        if let Some(path) = custom_path {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            let p = PathBuf::from(xdg).join("attack-shark-r1.ini");
            if p.exists() {
                return Some(p);
            }
        }

        if let Ok(home) = std::env::var("HOME") {
            let p = PathBuf::from(home).join(".config/attack-shark-r1.ini");
            if p.exists() {
                return Some(p);
            }
        }

        let etc = PathBuf::from("/etc/attack-shark-r1.ini");
        if etc.exists() {
            return Some(etc);
        }

        None
    }

    /// Loads configuration from an INI file.
    pub fn load_from_file(path: &Path) -> Result<Self, DriverError> {
        let conf = Ini::load_from_file(path)
            .map_err(|e| DriverError::Config(format!("Failed to parse INI file {}: {e}", path.display())))?;

        let mut cfg = MouseConfig::default();
        let section = conf.general_section();

        if let Some(val) = section.get("polling_rate") {
            let hz: u32 = val
                .trim()
                .parse()
                .map_err(|_| DriverError::Config(format!("Invalid polling rate in config: {val}")))?;
            cfg.polling_rate = PollingRate::try_from(hz)?;
        }

        if let Some(val) = section.get("sleep_time") {
            cfg.sleep_time = f64::from_str(val.trim())
                .map_err(|_| DriverError::Config(format!("Invalid sleep_time in config: {val}")))?;
        }

        if let Some(val) = section.get("deep_sleep_time") {
            cfg.deep_sleep_time = u8::from_str(val.trim())
                .map_err(|_| DriverError::Config(format!("Invalid deep_sleep_time in config: {val}")))?;
        }

        if let Some(val) = section.get("key_response_time") {
            cfg.key_response_time = u8::from_str(val.trim())
                .map_err(|_| DriverError::Config(format!("Invalid key_response_time in config: {val}")))?;
        }

        if let Some(val) = section.get("active_dpi") {
            cfg.active_dpi = u8::from_str(val.trim())
                .map_err(|_| DriverError::Config(format!("Invalid active_dpi in config: {val}")))?;
        }

        if let Some(val) = section.get("dpis") {
            let parts: Vec<&str> = val.split_whitespace().collect();
            if parts.len() != 6 {
                return Err(DriverError::Config(format!(
                    "Expected exactly 6 DPI values in 'dpis', found {}",
                    parts.len()
                )));
            }
            for (i, p) in parts.iter().enumerate() {
                let dpi: u32 = p
                    .parse()
                    .map_err(|_| DriverError::Config(format!("Invalid DPI number '{p}'")))?;
                cfg.dpis[i] = dpi;
            }
        }

        if let Some(val) = section.get("ripple_control") {
            cfg.ripple_control = bool::from_str(val.trim())
                .map_err(|_| DriverError::Config(format!("Invalid ripple_control in config: {val}")))?;
        }

        if let Some(val) = section.get("angle_snap") {
            cfg.angle_snap = bool::from_str(val.trim())
                .map_err(|_| DriverError::Config(format!("Invalid angle_snap in config: {val}")))?;
        }

        cfg.validate()?;
        Ok(cfg)
    }

    /// Saves configuration to an INI file.
    pub fn save_to_file(&self, path: &Path) -> Result<(), DriverError> {
        let mut conf = Ini::new();
        conf.with_general_section()
            .set("polling_rate", self.polling_rate.as_hz().to_string())
            .set("sleep_time", self.sleep_time.to_string())
            .set("deep_sleep_time", self.deep_sleep_time.to_string())
            .set("key_response_time", self.key_response_time.to_string())
            .set(
                "dpis",
                self.dpis
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            )
            .set("active_dpi", self.active_dpi.to_string())
            .set("ripple_control", self.ripple_control.to_string())
            .set("angle_snap", self.angle_snap.to_string());

        conf.write_to_file(path)
            .map_err(|e| DriverError::Config(format!("Failed to write config {}: {e}", path.display())))?;
        Ok(())
    }
}
