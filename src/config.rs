use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::error::DriverError;
use crate::protocol::PollingRate;

#[derive(Debug, Clone, PartialEq)]
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

    /// Finds the INI configuration file in standard search locations.
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
                "polling_rate" => {
                    let hz: u32 = val
                        .parse()
                        .map_err(|_| DriverError::Config(format!("Invalid polling rate '{val}' at line {}", line_num + 1)))?;
                    cfg.polling_rate = PollingRate::try_from(hz)?;
                }
                "sleep_time" => {
                    cfg.sleep_time = f64::from_str(val)
                        .map_err(|_| DriverError::Config(format!("Invalid sleep_time '{val}' at line {}", line_num + 1)))?;
                }
                "deep_sleep_time" => {
                    cfg.deep_sleep_time = u8::from_str(val)
                        .map_err(|_| DriverError::Config(format!("Invalid deep_sleep_time '{val}' at line {}", line_num + 1)))?;
                }
                "key_response_time" => {
                    cfg.key_response_time = u8::from_str(val)
                        .map_err(|_| DriverError::Config(format!("Invalid key_response_time '{val}' at line {}", line_num + 1)))?;
                }
                "active_dpi" => {
                    cfg.active_dpi = u8::from_str(val)
                        .map_err(|_| DriverError::Config(format!("Invalid active_dpi '{val}' at line {}", line_num + 1)))?;
                }
                "dpis" => {
                    let parts: Vec<&str> = val.split_whitespace().collect();
                    if parts.len() != 6 {
                        return Err(DriverError::Config(format!(
                            "Expected exactly 6 DPI values in 'dpis', found {} at line {}",
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
                "ripple_control" => {
                    cfg.ripple_control = bool::from_str(val)
                        .map_err(|_| DriverError::Config(format!("Invalid ripple_control '{val}' at line {}", line_num + 1)))?;
                }
                "angle_snap" => {
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
            "[general]\n\
             polling_rate = {}\n\
             sleep_time = {}\n\
             deep_sleep_time = {}\n\
             key_response_time = {}\n\
             dpis = {}\n\
             active_dpi = {}\n\
             ripple_control = {}\n\
             angle_snap = {}\n",
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

