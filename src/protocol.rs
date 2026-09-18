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

// Compile the DDSL hardware register specification into safe, typed Rust structs.
device_driver::compile!(
    manifest: "attack_shark_r1.ddsl"
);

/// Mouse polling rate options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum PollingRate {
    Hz125 = 0xf708,
    Hz250 = 0xfb04,
    Hz500 = 0xfd02,
    Hz1000 = 0xfe01,
}

impl PollingRate {
    pub fn as_hz(&self) -> u32 {
        match self {
            PollingRate::Hz125 => 125,
            PollingRate::Hz250 => 250,
            PollingRate::Hz500 => 500,
            PollingRate::Hz1000 => 1000,
        }
    }
}

impl TryFrom<u32> for PollingRate {
    type Error = DriverError;

    fn try_from(val: u32) -> Result<Self, Self::Error> {
        match val {
            125 => Ok(PollingRate::Hz125),
            250 => Ok(PollingRate::Hz250),
            500 => Ok(PollingRate::Hz500),
            1000 => Ok(PollingRate::Hz1000),
            other => Err(DriverError::InvalidPollingRate(other)),
        }
    }
}

impl fmt::Display for PollingRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}Hz", self.as_hz())
    }
}

/// Transport implementation binding `device-driver` traits to USB HID transfers.
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
            0x03 => {
                // Battery report
                if !self.is_wired {
                    let mut buf = [0u8; 64];
                    let read_len = self.handle.read_interrupt(
                        ENDPOINT_INTERRUPT_IN,
                        &mut buf,
                        Duration::from_millis(1500),
                    )?;
                    if read_len >= data.len() {
                        data.copy_from_slice(&buf[..data.len()]);
                    } else {
                        return Err(DriverError::Protocol(format!(
                            "Expected at least {} bytes on EP 0x83, got {}",
                            data.len(),
                            read_len
                        )));
                    }
                } else {
                    // When wired, mouse is connected via USB cable (charging/full power)
                    if data.len() >= 5 {
                        data[0] = 0x03;
                        data[1] = 0x10;
                        data[2] = 0x40;
                        data[3] = 0x01;
                        data[4] = 10; // 10 * 10 = 100%
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
        // HID SET_REPORT feature request (RequestType: Class | Interface | Out = 0x21, Request: 0x09)
        // wValue: (ReportType Feature (0x03) << 8) | ReportID (address)
        let w_value = (0x0300) | (address as u16);
        self.handle.write_control(
            0x21,
            0x09,
            w_value,
            INTERFACE_NUMBER as u16,
            data,
            Duration::from_millis(1500),
        )?;

        // If wireless, the mouse confirms receipt on interrupt endpoint 0x83
        if !self.is_wired {
            let mut ack_buf = [0u8; 5];
            let _ = self.handle.read_interrupt(
                ENDPOINT_INTERRUPT_IN,
                &mut ack_buf,
                Duration::from_millis(800),
            );
        }

        Ok(())
    }
}
