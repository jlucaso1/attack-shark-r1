#[cfg(feature = "polling-rate")]
use std::fmt;
use std::time::Duration;

use device_driver::{FieldsetMetadata, RegisterInterface, RegisterInterfaceBase};
use rusb::{DeviceHandle, GlobalContext};

use crate::error::DriverError;

pub const VENDOR_ID: u16 = 0x1d57;
pub const PRODUCT_ID_WIRELESS: u16 = 0xfa60;
pub const PRODUCT_ID_WIRED: u16 = 0xfa61;
pub const INTERFACE_NUMBER: u8 = 2;
pub const ENDPOINT_INTERRUPT_IN: u8 = 0x83;

pub const REGISTER_BATTERY: u8 = 0x03;
pub const REGISTER_DPI: u8 = 0x04;
pub const REGISTER_SLEEP_TIMES: u8 = 0x05;
pub const REGISTER_POLLING_RATE: u8 = 0x06;

pub const DEVICE_ID_ATTACK_SHARK_R1: u8 = 0x10;
pub const BATTERY_STATUS_READY: u8 = 0x40;
pub const BATTERY_FLAG_DISCHARGING: u8 = 0x01;
pub const BATTERY_FLAG_CHARGING_WIRED: u8 = 0x02;
pub const BATTERY_FLAG_CHARGING_WIRELESS: u8 = 0x03;
pub const BATTERY_FLAG_CHARGING_DOCK: u8 = 0x80;
pub const BATTERY_MAX_TENTHS: u8 = 10;
pub const MIN_BATTERY_REPORT_LEN: usize = 5;
pub const INTERRUPT_BUFFER_LEN: usize = 64;

pub const USB_REQ_TYPE_IN_DEVICE: u8 = 0x80;
pub const USB_REQ_GET_DESCRIPTOR: u8 = 0x06;
pub const USB_DESC_TYPE_DEVICE: u16 = 0x0100;
pub const USB_DEVICE_DESC_LEN: usize = 18;

pub const USB_REQ_TYPE_CLASS_INTERFACE_OUT: u8 = 0x21;
pub const USB_HID_REQ_SET_REPORT: u8 = 0x09;
pub const USB_HID_REPORT_TYPE_FEATURE: u16 = 0x0300;

pub const TIMEOUT_INTERRUPT: Duration = Duration::from_millis(1500);
pub const TIMEOUT_CONTROL: Duration = Duration::from_millis(1500);
pub const TIMEOUT_LIVENESS: Duration = Duration::from_millis(200);
pub const TIMEOUT_WIRED_LIVENESS: Duration = Duration::from_millis(500);
pub const TIMEOUT_ACK: Duration = Duration::from_millis(800);

#[cfg(feature = "polling-rate")]
pub const POLLING_RATE_125_HZ: u32 = 125;
#[cfg(feature = "polling-rate")]
pub const POLLING_RATE_250_HZ: u32 = 250;
#[cfg(feature = "polling-rate")]
pub const POLLING_RATE_500_HZ: u32 = 500;
#[cfg(feature = "polling-rate")]
pub const POLLING_RATE_1000_HZ: u32 = 1000;

// Compile DDSL hardware register specification.
device_driver::compile!(
    manifest: "attack_shark_r1.ddsl"
);

/// Polling rate options.
#[cfg(feature = "polling-rate")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum PollingRate {
    Hz125 = 0xf708,
    Hz250 = 0xfb04,
    Hz500 = 0xfd02,
    Hz1000 = 0xfe01,
}

#[cfg(feature = "polling-rate")]
impl PollingRate {
    pub fn as_hz(&self) -> u32 {
        match self {
            PollingRate::Hz125 => POLLING_RATE_125_HZ,
            PollingRate::Hz250 => POLLING_RATE_250_HZ,
            PollingRate::Hz500 => POLLING_RATE_500_HZ,
            PollingRate::Hz1000 => POLLING_RATE_1000_HZ,
        }
    }
}

#[cfg(feature = "polling-rate")]
impl TryFrom<u32> for PollingRate {
    type Error = DriverError;

    fn try_from(val: u32) -> Result<Self, Self::Error> {
        match val {
            POLLING_RATE_125_HZ => Ok(PollingRate::Hz125),
            POLLING_RATE_250_HZ => Ok(PollingRate::Hz250),
            POLLING_RATE_500_HZ => Ok(PollingRate::Hz500),
            POLLING_RATE_1000_HZ => Ok(PollingRate::Hz1000),
            other => Err(DriverError::InvalidPollingRate(other)),
        }
    }
}

#[cfg(feature = "polling-rate")]
impl fmt::Display for PollingRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}Hz", self.as_hz())
    }
}

/// USB HID transport for device-driver.
pub struct UsbTransport {
    pub(crate) handle: DeviceHandle<GlobalContext>,
    pub(crate) is_wired: bool,
    kernel_driver_was_active: bool,
}

impl UsbTransport {
    pub fn new(handle: DeviceHandle<GlobalContext>, is_wired: bool) -> Result<Self, DriverError> {
        let kernel_driver_was_active = handle.kernel_driver_active(INTERFACE_NUMBER).unwrap_or(false);
        if kernel_driver_was_active {
            let _ = handle.detach_kernel_driver(INTERFACE_NUMBER);
        }

        handle
            .claim_interface(INTERFACE_NUMBER)
            .map_err(DriverError::ClaimInterface)?;

        Ok(Self {
            handle,
            is_wired,
            kernel_driver_was_active,
        })
    }

    pub fn is_wired(&self) -> bool {
        self.is_wired
    }

    /// Probes the USB connection to verify the device is still physically connected.
    pub fn check_liveness(&mut self, timeout: Duration) -> Result<(), DriverError> {
        let mut desc_buf = [0u8; USB_DEVICE_DESC_LEN];
        self.handle.read_control(
            USB_REQ_TYPE_IN_DEVICE,
            USB_REQ_GET_DESCRIPTOR,
            USB_DESC_TYPE_DEVICE,
            0,
            &mut desc_buf,
            timeout,
        )?;
        Ok(())
    }
}

impl Drop for UsbTransport {
    fn drop(&mut self) {
        let _ = self.handle.release_interface(INTERFACE_NUMBER);
        if self.kernel_driver_was_active {
            let _ = self.handle.attach_kernel_driver(INTERFACE_NUMBER);
        }
    }
}

impl RegisterInterfaceBase for UsbTransport {
    type Error = DriverError;
    type AddressType = u8;
}

impl RegisterInterface for UsbTransport {
    fn read_register(
        &mut self,
        address: Self::AddressType,
        data: &mut [u8],
        _metadata: &FieldsetMetadata,
    ) -> Result<(), Self::Error> {
        match address {
            REGISTER_BATTERY => {
                if !self.is_wired {
                    let mut buf = [0u8; INTERRUPT_BUFFER_LEN];
                    let read_len = match self.handle.read_interrupt(
                        ENDPOINT_INTERRUPT_IN,
                        &mut buf,
                        TIMEOUT_INTERRUPT,
                    ) {
                        Ok(n) => n,
                        Err(rusb::Error::Timeout) => {
                            // Dongle received no packet over RF (mouse is asleep).
                            // Verify that the dongle itself is still connected to USB.
                            self.check_liveness(TIMEOUT_LIVENESS)?;
                            return Err(DriverError::Usb(rusb::Error::Timeout));
                        }
                        Err(e) => return Err(DriverError::Usb(e)),
                    };
                    data.fill(0);
                    if read_len >= MIN_BATTERY_REPORT_LEN {
                        let copy_len = read_len.min(data.len());
                        data[..copy_len].copy_from_slice(&buf[..copy_len]);
                    } else {
                        return Err(DriverError::Protocol(format!(
                            "Expected at least {MIN_BATTERY_REPORT_LEN} bytes on EP 0x{ENDPOINT_INTERRUPT_IN:02x}, got {read_len}"
                        )));
                    }
                } else {
                    // Verify that the wired USB mouse is still physically connected.
                    self.check_liveness(TIMEOUT_WIRED_LIVENESS)?;

                    data.fill(0);
                    if data.len() >= MIN_BATTERY_REPORT_LEN {
                        data[0] = REGISTER_BATTERY;
                        data[1] = DEVICE_ID_ATTACK_SHARK_R1;
                        data[2] = BATTERY_STATUS_READY;
                        data[3] = BATTERY_FLAG_CHARGING_WIRED;
                        data[4] = BATTERY_MAX_TENTHS;
                    }
                }
                Ok(())
            }
            other => Err(DriverError::Protocol(format!(
                "Read not supported for register 0x{other:02x}"
            ))),
        }
    }

    fn write_register(
        &mut self,
        address: Self::AddressType,
        data: &mut [u8],
        _metadata: &FieldsetMetadata,
    ) -> Result<(), Self::Error> {
        let w_value = USB_HID_REPORT_TYPE_FEATURE | (address as u16);
        self.handle.write_control(
            USB_REQ_TYPE_CLASS_INTERFACE_OUT,
            USB_HID_REQ_SET_REPORT,
            w_value,
            INTERFACE_NUMBER as u16,
            data,
            TIMEOUT_CONTROL,
        )?;

        // Wireless mouse confirms writes on interrupt endpoint
        if !self.is_wired {
            let mut ack_buf = [0u8; MIN_BATTERY_REPORT_LEN];
            let _ = self.handle.read_interrupt(
                ENDPOINT_INTERRUPT_IN,
                &mut ack_buf,
                TIMEOUT_ACK,
            );
        }

        Ok(())
    }
}
