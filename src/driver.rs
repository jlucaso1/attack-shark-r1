use device_driver::{Block, FieldsetMetadata, RegisterInterface};
use serde::{Deserialize, Serialize};

use crate::config::MouseConfig;
use crate::dpi::dpi_to_byte;
use crate::error::DriverError;
use crate::protocol::{
    AttackSharkR1Device, PollingRate, UsbTransport, PRODUCT_ID_WIRED, PRODUCT_ID_WIRELESS,
    VENDOR_ID,
};

/// High-level representation of the mouse battery status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryStatus {
    /// Battery charge percentage (0-100%).
    pub percentage: u8,
    /// Whether the mouse is connected via USB cable.
    pub is_wired: bool,
    /// Raw charge value reported by the mouse (0-10).
    pub raw_charge: u8,
    /// Raw status byte.
    pub status_code: u8,
    /// HID Report ID (0x03).
    pub report_id: u8,
}

/// Attack Shark R1 mouse driver.
pub struct AttackSharkR1 {
    device: AttackSharkR1Device<UsbTransport>,
}

impl AttackSharkR1 {
    /// Discovers and opens the Attack Shark R1 mouse (either wireless 2.4G dongle or wired).
    pub fn open() -> Result<Self, DriverError> {
        let devices = rusb::devices()?;
        let mut target = None;

        for dev in devices.iter() {
            let desc = dev.device_descriptor()?;
            if desc.vendor_id() == VENDOR_ID
                && (desc.product_id() == PRODUCT_ID_WIRELESS
                    || desc.product_id() == PRODUCT_ID_WIRED)
            {
                let is_wired = desc.product_id() == PRODUCT_ID_WIRED;
                target = Some((dev, is_wired));
                break;
            }
        }

        let (device, is_wired) = target.ok_or(DriverError::DeviceNotFound)?;
        let handle = device.open()?;
        let transport = UsbTransport::new(handle, is_wired)?;
        let device_instance = AttackSharkR1Device::new(transport);

        Ok(Self {
            device: device_instance,
        })
    }

    /// Returns whether the mouse is currently connected wired.
    pub fn is_wired(&mut self) -> bool {
        self.device.interface().is_wired()
    }

    /// Queries the current battery charge percentage (0..=100%).
    pub fn get_battery_percentage(&mut self) -> Result<u8, DriverError> {
        let report = self.device.battery_report().read()?;
        let raw = report.charge_raw();
        // The mouse reports charge from 0 to 10 (representing 0% to 100%)
        let pct = (raw as u16 * 10).min(100) as u8;
        Ok(pct)
    }

    /// Queries detailed battery status including raw values and connection type.
    pub fn get_battery_status(&mut self) -> Result<BatteryStatus, DriverError> {
        let report = self.device.battery_report().read()?;
        let raw = report.charge_raw();
        let pct = (raw as u16 * 10).min(100) as u8;
        let is_wired = self.device.interface().is_wired();

        Ok(BatteryStatus {
            percentage: pct,
            is_wired,
            raw_charge: raw,
            status_code: report.status(),
            report_id: report.report_id(),
        })
    }

    /// Sets the mouse USB polling rate (125Hz, 250Hz, 500Hz, or 1000Hz).
    pub fn set_polling_rate(&mut self, rate: PollingRate) -> Result<(), DriverError> {
        let rate_val = rate as u16;
        let rate_bytes = rate_val.to_le_bytes();

        let mut payload = [0u8; 9];
        payload[0] = 0x06; // Report ID
        payload[1] = 0x09; // Command
        payload[2] = 0x01; // Sub
        payload[3] = rate_bytes[0];
        payload[4] = rate_bytes[1];

        self.device.interface().write_register(
            0x06,
            &mut payload,
            &FieldsetMetadata::new(),
        )?;

        Ok(())
    }

    /// Sets power saving and debouncing times:
    /// - `sleep_time_seconds`: 0.5 to 30.0 seconds
    /// - `deep_sleep_minutes`: 1 to 60 minutes
    /// - `debounce_ms`: 4 to 50 ms (must be even)
    pub fn set_sleep_times(
        &mut self,
        sleep_time_seconds: f64,
        deep_sleep_minutes: u8,
        debounce_ms: u8,
    ) -> Result<(), DriverError> {
        if sleep_time_seconds < 0.5 || sleep_time_seconds > 30.0 {
            return Err(DriverError::InvalidSleepTime(sleep_time_seconds));
        }
        if deep_sleep_minutes < 1 || deep_sleep_minutes > 60 {
            return Err(DriverError::InvalidDeepSleepTime(deep_sleep_minutes));
        }
        if debounce_ms < 4 || debounce_ms > 50 || debounce_ms % 2 != 0 {
            return Err(DriverError::InvalidKeyResponseTime(debounce_ms));
        }

        let mut payload = [
            0x05, 0x0f, 0x01, 0x00, 0x03, 0x18, 0x00, 0x00, 0xff, 0x04, 0x02, 0x01, 0x20, 0x00,
            0x00,
        ];

        payload[4] = 0x03 | (deep_sleep_minutes & 0xF0);
        payload[5] = 0x08 | ((deep_sleep_minutes & 0x0F) << 4);
        payload[9] = (sleep_time_seconds * 2.0) as u8;
        payload[10] = debounce_ms / 2;

        let ds_low = deep_sleep_minutes & 0x0F;
        let ds_high = (deep_sleep_minutes >> 4) & 0x0F;
        let checksum = (((ds_low + ds_high) & 0x0F) << 4) + 0x0a + payload[9] + payload[10];
        payload[12] = checksum;

        self.device.interface().write_register(
            0x05,
            &mut payload,
            &FieldsetMetadata::new(),
        )?;

        Ok(())
    }

    /// Configures the 6 DPI stages, active stage, and angle snap / ripple control features.
    pub fn set_dpi_profile(
        &mut self,
        dpis: [u32; 6],
        active_stage: u8,
        angle_snap: bool,
        ripple_control: bool,
    ) -> Result<(), DriverError> {
        if active_stage < 1 || active_stage > 6 {
            return Err(DriverError::InvalidDpiStage(active_stage));
        }

        let mut payload = [
            0x04, 0x38, 0x01, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xff, 0x00, 0x00,
            0x00, 0xff, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0x00, 0x00, 0xff, 0xff, 0xff, 0x00,
            0xff, 0xff, 0x40, 0x00, 0xff, 0xff, 0xff, 0x02, 0x0d, 0x75, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut checksum = 0x0d75u16;
        let mut is_bigger_than_12k = 0u8;

        for (i, &dpi) in dpis.iter().enumerate() {
            let code = dpi_to_byte(dpi)?;
            payload[i + 8] = code;
            checksum = checksum.wrapping_add(code as u16);

            let is_10k_to_12k = if (10100..=12000).contains(&dpi) {
                1u8
            } else {
                0u8
            };
            payload[i + 16] = is_10k_to_12k;
            checksum = checksum.wrapping_add(is_10k_to_12k as u16);

            if dpi > 12000 {
                is_bigger_than_12k |= 1 << (i as u8);
            }
        }

        payload[6] = is_bigger_than_12k;
        payload[7] = is_bigger_than_12k;
        checksum = checksum.wrapping_add((is_bigger_than_12k as u16).wrapping_mul(2));

        payload[24] = active_stage;
        checksum = checksum.wrapping_add((active_stage - 1) as u16);

        if ripple_control {
            checksum = checksum.wrapping_add(1);
            payload[4] = 1;
        }

        if angle_snap {
            checksum = checksum.wrapping_add(1);
            payload[3] = 1;
        }

        let chk_bytes = checksum.to_be_bytes();
        payload[50] = chk_bytes[0];
        payload[51] = chk_bytes[1];

        self.device.interface().write_register(
            0x04,
            &mut payload,
            &FieldsetMetadata::new(),
        )?;

        Ok(())
    }

    /// Applies a full `MouseConfig` to the hardware.
    pub fn apply_config(&mut self, config: &MouseConfig) -> Result<(), DriverError> {
        config.validate()?;
        self.set_polling_rate(config.polling_rate)?;
        self.set_sleep_times(
            config.sleep_time,
            config.deep_sleep_time,
            config.key_response_time,
        )?;
        self.set_dpi_profile(
            config.dpis,
            config.active_dpi,
            config.angle_snap,
            config.ripple_control,
        )?;
        Ok(())
    }
}
