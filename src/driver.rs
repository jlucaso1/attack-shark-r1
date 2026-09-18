use device_driver::Block;
#[cfg(any(feature = "polling-rate", feature = "sleep", feature = "dpi"))]
use device_driver::{FieldsetMetadata, RegisterInterface};

#[cfg(feature = "config")]
use crate::config::MouseConfig;
#[cfg(feature = "dpi")]
use crate::dpi::dpi_to_byte;
use crate::error::DriverError;
#[cfg(feature = "polling-rate")]
use crate::protocol::PollingRate;
use crate::protocol::{
    AttackSharkR1Device, UsbTransport, BATTERY_FLAG_CHARGING_DOCK,
    BATTERY_FLAG_CHARGING_WIRED, BATTERY_FLAG_CHARGING_WIRELESS, PRODUCT_ID_WIRED,
    PRODUCT_ID_WIRELESS, VENDOR_ID,
};
#[cfg(feature = "dpi")]
use crate::protocol::REGISTER_DPI;
#[cfg(feature = "polling-rate")]
use crate::protocol::REGISTER_POLLING_RATE;
#[cfg(feature = "sleep")]
use crate::protocol::REGISTER_SLEEP_TIMES;

#[cfg(feature = "sleep")]
pub const SLEEP_TIME_MIN: f64 = 0.5;
#[cfg(feature = "sleep")]
pub const SLEEP_TIME_MAX: f64 = 30.0;
#[cfg(feature = "sleep")]
pub const DEEP_SLEEP_MIN: u8 = 1;
#[cfg(feature = "sleep")]
pub const DEEP_SLEEP_MAX: u8 = 60;
#[cfg(feature = "sleep")]
pub const DEBOUNCE_MIN: u8 = 4;
#[cfg(feature = "sleep")]
pub const DEBOUNCE_MAX: u8 = 50;

/// Validates sleep timing parameters against hardware capabilities.
#[cfg(feature = "sleep")]
pub fn validate_sleep_times(
    sleep_time_seconds: f64,
    deep_sleep_minutes: u8,
    debounce_ms: u8,
) -> Result<(), DriverError> {
    if !(SLEEP_TIME_MIN..=SLEEP_TIME_MAX).contains(&sleep_time_seconds) {
        return Err(DriverError::InvalidSleepTime(sleep_time_seconds));
    }
    if !(DEEP_SLEEP_MIN..=DEEP_SLEEP_MAX).contains(&deep_sleep_minutes) {
        return Err(DriverError::InvalidDeepSleepTime(deep_sleep_minutes));
    }
    if !(DEBOUNCE_MIN..=DEBOUNCE_MAX).contains(&debounce_ms) || !debounce_ms.is_multiple_of(2) {
        return Err(DriverError::InvalidKeyResponseTime(debounce_ms));
    }
    Ok(())
}

const MAX_PERCENTAGE: u8 = 100;
const PERCENTAGE_TENTHS_SCALE: u16 = 10;
const MAX_TENTHS_RAW: u8 = 10;

#[cfg(feature = "polling-rate")]
const CMD_POLLING_RATE: u8 = 0x09;
#[cfg(feature = "polling-rate")]
const SUB_POLLING_RATE: u8 = 0x01;

#[cfg(feature = "sleep")]
const CMD_SLEEP_TIMES: u8 = 0x0f;
#[cfg(feature = "sleep")]
const SUB_SLEEP_TIMES: u8 = 0x01;

#[cfg(feature = "dpi")]
const CMD_DPI: u8 = 0x38;
#[cfg(feature = "dpi")]
const SUB_DPI: u8 = 0x01;
#[cfg(feature = "dpi")]
const DPI_MID_BAND_MIN: u32 = 10100;
#[cfg(feature = "dpi")]
const DPI_MID_BAND_MAX: u32 = 12000;
#[cfg(feature = "dpi")]
const DPI_HIGH_BAND_THRESHOLD: u32 = 12000;
#[cfg(feature = "dpi")]
const DPI_BASE_CHECKSUM: u16 = 0x0d75;

/// Mouse battery status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatteryStatus {
    /// Battery charge percentage (0..=100).
    pub percentage: u8,
    /// Whether the mouse is charging.
    pub is_charging: bool,
    /// True if connected with a USB-C cable.
    pub is_wired: bool,
    /// Raw charge value from hardware.
    pub raw_charge: u8,
    /// Raw status byte.
    pub status_code: u8,
    /// Protocol flags byte.
    pub flags: u8,
    /// HID Report ID (0x03).
    pub report_id: u8,
}

/// Attack Shark R1 driver.
pub struct AttackSharkR1 {
    device: AttackSharkR1Device<UsbTransport>,
}

impl AttackSharkR1 {
    /// Opens the mouse via 2.4G wireless dongle or USB-C cable.
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

    /// Returns true if connected with a USB cable.
    pub fn is_wired(&mut self) -> bool {
        self.device.interface().is_wired()
    }

    /// Reads battery charge percentage (0..=100).
    pub fn get_battery_percentage(&mut self) -> Result<u8, DriverError> {
        let status = self.get_battery_status()?;
        Ok(status.percentage)
    }

    /// Reads battery percentage, charging state, and connection type.
    pub fn get_battery_status(&mut self) -> Result<BatteryStatus, DriverError> {
        let report = self.device.battery_report().read()?;
        let raw = report.charge_raw();
        // Mouse reports tenths (0..=10) or raw percentage (0..=100)
        let pct = if raw <= MAX_TENTHS_RAW {
            (raw as u16 * PERCENTAGE_TENTHS_SCALE).min(MAX_PERCENTAGE as u16) as u8
        } else {
            raw.min(MAX_PERCENTAGE)
        };
        let is_wired = self.device.interface().is_wired();
        let flags = report.flags();
        let flag1 = report.flag_1();
        let flag2 = report.flag_2();
        let flag3 = report.flag_3();

        let is_charging = is_wired
            || matches!(
                flags,
                BATTERY_FLAG_CHARGING_WIRED
                    | BATTERY_FLAG_CHARGING_WIRELESS
                    | BATTERY_FLAG_CHARGING_DOCK
            )
            || flag1 != 0
            || flag2 != 0
            || flag3 != 0;

        Ok(BatteryStatus {
            percentage: pct,
            is_charging,
            is_wired,
            raw_charge: raw,
            status_code: report.status(),
            flags,
            report_id: report.report_id(),
        })
    }

    /// Sets USB polling rate (125, 250, 500, or 1000 Hz).
    #[cfg(feature = "polling-rate")]
    pub fn set_polling_rate(&mut self, rate: PollingRate) -> Result<(), DriverError> {
        let rate_val = rate as u16;
        let rate_bytes = rate_val.to_le_bytes();

        let mut payload = [0u8; 9];
        payload[0] = REGISTER_POLLING_RATE;
        payload[1] = CMD_POLLING_RATE;
        payload[2] = SUB_POLLING_RATE;
        payload[3] = rate_bytes[0];
        payload[4] = rate_bytes[1];

        self.device.interface().write_register(
            REGISTER_POLLING_RATE,
            &mut payload,
            &FieldsetMetadata::new(),
        )?;

        Ok(())
    }

    /// Sets sleep delay (0.5 to 30.0 s), deep sleep (1 to 60 min), and debounce (4 to 50 ms).
    #[cfg(feature = "sleep")]
    pub fn set_sleep_times(
        &mut self,
        sleep_time_seconds: f64,
        deep_sleep_minutes: u8,
        debounce_ms: u8,
    ) -> Result<(), DriverError> {
        validate_sleep_times(sleep_time_seconds, deep_sleep_minutes, debounce_ms)?;

        let mut payload = [
            0x05, 0x0f, 0x01, 0x00, 0x03, 0x18, 0x00, 0x00, 0xff, 0x04, 0x02, 0x01, 0x20, 0x00,
            0x00,
        ];

        payload[0] = REGISTER_SLEEP_TIMES;
        payload[1] = CMD_SLEEP_TIMES;
        payload[2] = SUB_SLEEP_TIMES;
        payload[4] = 0x03 | (deep_sleep_minutes & 0xF0);
        payload[5] = 0x08 | ((deep_sleep_minutes & 0x0F) << 4);
        payload[9] = (sleep_time_seconds * 2.0) as u8;
        payload[10] = debounce_ms / 2;

        let ds_low = deep_sleep_minutes & 0x0F;
        let ds_high = (deep_sleep_minutes >> 4) & 0x0F;
        let checksum = (((ds_low + ds_high) & 0x0F) << 4) + 0x0a + payload[9] + payload[10];
        payload[12] = checksum;

        self.device.interface().write_register(
            REGISTER_SLEEP_TIMES,
            &mut payload,
            &FieldsetMetadata::new(),
        )?;

        Ok(())
    }

    /// Sets the 6 DPI stages, active stage, angle snap, and ripple control.
    #[cfg(feature = "dpi")]
    pub fn set_dpi_profile(
        &mut self,
        dpis: [u32; 6],
        active_stage: u8,
        angle_snap: bool,
        ripple_control: bool,
    ) -> Result<(), DriverError> {
        crate::dpi::validate_dpi_stage(active_stage)?;

        let mut payload = [
            0x04, 0x38, 0x01, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xff, 0x00, 0x00,
            0x00, 0xff, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0x00, 0x00, 0xff, 0xff, 0xff, 0x00,
            0xff, 0xff, 0x40, 0x00, 0xff, 0xff, 0xff, 0x02, 0x0d, 0x75, 0x00, 0x00, 0x00, 0x00,
        ];

        payload[0] = REGISTER_DPI;
        payload[1] = CMD_DPI;
        payload[2] = SUB_DPI;

        let mut checksum = DPI_BASE_CHECKSUM;
        let mut is_bigger_than_12k = 0u8;

        for (i, &dpi) in dpis.iter().enumerate() {
            let code = dpi_to_byte(dpi)?;
            payload[i + 8] = code;
            checksum = checksum.wrapping_add(code as u16);

            let is_mid_band = if (DPI_MID_BAND_MIN..=DPI_MID_BAND_MAX).contains(&dpi) {
                1u8
            } else {
                0u8
            };
            payload[i + 16] = is_mid_band;
            checksum = checksum.wrapping_add(is_mid_band as u16);

            if dpi > DPI_HIGH_BAND_THRESHOLD {
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
            REGISTER_DPI,
            &mut payload,
            &FieldsetMetadata::new(),
        )?;

        Ok(())
    }

    /// Applies an INI configuration to hardware registers.
    #[cfg(feature = "config")]
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
