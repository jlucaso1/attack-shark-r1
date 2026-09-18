use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::dpi::{validate_dpi, validate_dpi_stage, DPI_STAGES_COUNT};
use crate::driver::validate_sleep_times;
use crate::error::DriverError;
use crate::protocol::PollingRate;

pub const CONFIG_FILE_NAME: &str = "attack-shark-r1.ini";
pub const ETC_CONFIG_PATH: &str = "/etc/attack-shark-r1.ini";
pub const XDG_CONFIG_ENV: &str = "XDG_CONFIG_HOME";
pub const HOME_ENV: &str = "HOME";

pub const SECTION_GENERAL: &str = "[general]";
pub const KEY_POLLING_RATE: &str = "polling_rate";
pub const KEY_SLEEP_TIME: &str = "sleep_time";
pub const KEY_DEEP_SLEEP_TIME: &str = "deep_sleep_time";
pub const KEY_KEY_RESPONSE_TIME: &str = "key_response_time";
pub const KEY_ACTIVE_DPI: &str = "active_dpi";
pub const KEY_DPIS: &str = "dpis";
pub const KEY_RIPPLE_CONTROL: &str = "ripple_control";
pub const KEY_ANGLE_SNAP: &str = "angle_snap";

#[derive(Debug, Clone, PartialEq)]
pub struct MouseConfig {
    pub polling_rate: PollingRate,
    pub sleep_time: f64,
    pub deep_sleep_time: u8,
    pub key_response_time: u8,
    pub dpis: [u32; DPI_STAGES_COUNT],
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
        validate_sleep_times(
            self.sleep_time,
            self.deep_sleep_time,
            self.key_response_time,
        )?;
        validate_dpi_stage(self.active_dpi)?;
        for &dpi in &self.dpis {
            validate_dpi(dpi)?;
        }
        Ok(())
    }

    /// Finds the INI configuration file in standard search locations.
    pub fn find_config_file(custom_path: Option<&str>) -> Option<PathBuf> {
        if let Some(path) = custom_path {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        if let Ok(xdg) = std::env::var(XDG_CONFIG_ENV) {
            let p = PathBuf::from(xdg).join(CONFIG_FILE_NAME);
            if p.exists() {
                return Some(p);
            }
        }

        if let Ok(home) = std::env::var(HOME_ENV) {
            let p = PathBuf::from(home).join(".config").join(CONFIG_FILE_NAME);
            if p.exists() {
                return Some(p);
            }
        }

        let etc = PathBuf::from(ETC_CONFIG_PATH);
        if etc.exists() {
            return Some(etc);
        }

        None
    }

    /// Loads configuration from an INI file.
    pub fn load_from_file(path: &Path) -> Result<Self, DriverError> {
        let content = fs::read_to_string(path)
            .map_err(|e| DriverError::Config(format!("Failed to read INI file {}: {e}", path.display())))?;

        let mut cfg = MouseConfig::default();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            // Skip empty lines, section headers, and comments (# or ;)
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') || trimmed.starts_with('[') {
                continue;
            }

            let (key, val) = match trimmed.split_once('=') {
                Some((k, v)) => (k.trim(), v.trim()),
                None => continue,
            };

            // Remove inline comments if any
            let val = val.split('#').next().unwrap_or(val);
            let val = val.split(';').next().unwrap_or(val).trim();

            match key {
                KEY_POLLING_RATE => {
                    let hz: u32 = val
                        .parse()
                        .map_err(|_| DriverError::Config(format!("Invalid polling rate '{val}' at line {}", line_num + 1)))?;
                    cfg.polling_rate = PollingRate::try_from(hz)?;
                }
                KEY_SLEEP_TIME => {
                    cfg.sleep_time = f64::from_str(val)
                        .map_err(|_| DriverError::Config(format!("Invalid sleep_time '{val}' at line {}", line_num + 1)))?;
                }
                KEY_DEEP_SLEEP_TIME => {
                    cfg.deep_sleep_time = u8::from_str(val)
                        .map_err(|_| DriverError::Config(format!("Invalid deep_sleep_time '{val}' at line {}", line_num + 1)))?;
                }
                KEY_KEY_RESPONSE_TIME => {
                    cfg.key_response_time = u8::from_str(val)
                        .map_err(|_| DriverError::Config(format!("Invalid key_response_time '{val}' at line {}", line_num + 1)))?;
                }
                KEY_ACTIVE_DPI => {
                    cfg.active_dpi = u8::from_str(val)
                        .map_err(|_| DriverError::Config(format!("Invalid active_dpi '{val}' at line {}", line_num + 1)))?;
                }
                KEY_DPIS => {
                    let parts: Vec<&str> = val.split_whitespace().collect();
                    if parts.len() != DPI_STAGES_COUNT {
                        return Err(DriverError::Config(format!(
                            "Expected exactly {DPI_STAGES_COUNT} DPI values in '{KEY_DPIS}', found {} at line {}",
                            parts.len(),
                            line_num + 1
                        )));
                    }
                    for (i, p) in parts.iter().enumerate() {
                        let dpi: u32 = p
                            .parse()
                            .map_err(|_| DriverError::Config(format!("Invalid DPI number '{p}' at line {}", line_num + 1)))?;
                        cfg.dpis[i] = dpi;
                    }
                }
                KEY_RIPPLE_CONTROL => {
                    cfg.ripple_control = bool::from_str(val)
                        .map_err(|_| DriverError::Config(format!("Invalid ripple_control '{val}' at line {}", line_num + 1)))?;
                }
                KEY_ANGLE_SNAP => {
                    cfg.angle_snap = bool::from_str(val)
                        .map_err(|_| DriverError::Config(format!("Invalid angle_snap '{val}' at line {}", line_num + 1)))?;
                }
                _ => {}
            }
        }

        cfg.validate()?;
        Ok(cfg)
    }

    /// Saves configuration to an INI file without external dependencies.
    pub fn save_to_file(&self, path: &Path) -> Result<(), DriverError> {
        let dpis_str = self
            .dpis
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        let content = format!(
            "{SECTION_GENERAL}\n\
             {KEY_POLLING_RATE} = {}\n\
             {KEY_SLEEP_TIME} = {}\n\
             {KEY_DEEP_SLEEP_TIME} = {}\n\
             {KEY_KEY_RESPONSE_TIME} = {}\n\
             {KEY_DPIS} = {}\n\
             {KEY_ACTIVE_DPI} = {}\n\
             {KEY_RIPPLE_CONTROL} = {}\n\
             {KEY_ANGLE_SNAP} = {}\n",
            self.polling_rate.as_hz(),
            self.sleep_time,
            self.deep_sleep_time,
            self.key_response_time,
            dpis_str,
            self.active_dpi,
            self.ripple_control,
            self.angle_snap,
        );

        fs::write(path, content)
            .map_err(|e| DriverError::Config(format!("Failed to write config {}: {e}", path.display())))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parse() {
        let sample = r#"
        # Sample config
        [general]
        polling_rate = 500
        sleep_time = 10.5 ; comment
        deep_sleep_time = 20 # inline comment
        key_response_time = 8
        dpis = 400 800 1600 3200 6400 12000
        active_dpi = 4
        ripple_control = true
        angle_snap = true
        "#;

        let tmp = std::env::temp_dir().join("test_attack_shark_r1.ini");
        fs::write(&tmp, sample).unwrap();

        let cfg = MouseConfig::load_from_file(&tmp).unwrap();
        assert_eq!(cfg.polling_rate, PollingRate::Hz500);
        assert_eq!(cfg.sleep_time, 10.5);
        assert_eq!(cfg.deep_sleep_time, 20);
        assert_eq!(cfg.key_response_time, 8);
        assert_eq!(cfg.dpis, [400, 800, 1600, 3200, 6400, 12000]);
        assert_eq!(cfg.active_dpi, 4);
        assert!(cfg.ripple_control);
        assert!(cfg.angle_snap);

        let _ = fs::remove_file(tmp);
    }
}

