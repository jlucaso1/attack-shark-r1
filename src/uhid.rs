use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;

use crate::error::DriverError;

const UHID_PATH: &str = "/dev/uhid";
const BUS_USB: u16 = 0x03;

const UHID_DESTROY: u32 = 1;
const UHID_GET_REPORT: u32 = 9;
const UHID_GET_REPORT_REPLY: u32 = 10;
const UHID_CREATE2: u32 = 11;
const UHID_INPUT2: u32 = 12;

/// HID report descriptor exposing mouse battery percentage and charging state.
///
/// Includes a 1-bit dummy button so the kernel hid-input driver claims the device
/// and routes input events. Uses Battery System usages for charge percentage (0x65)
/// and charging status (0x44).
const BATTERY_REPORT_DESCRIPTOR: &[u8] = &[
    0x05, 0x01,        // Usage Page (Generic Desktop Ctrls)
    0x09, 0x02,        // Usage (Mouse)
    0xa1, 0x01,        // Collection (Application)
    0x85, 0x01,        //   Report ID (1)
    0x05, 0x09,        //   Usage Page (Button)
    0x09, 0x01,        //   Usage (Button 1)
    0x15, 0x00,        //   Logical Minimum (0)
    0x25, 0x01,        //   Logical Maximum (1)
    0x75, 0x01,        //   Report Size (1 bit)
    0x95, 0x01,        //   Report Count (1)
    0x81, 0x02,        //   Input (Data,Var,Abs)
    0x75, 0x07,        //   Report Size (7 bits padding)
    0x95, 0x01,        //   Report Count (1)
    0x81, 0x03,        //   Input (Const,Var,Abs)
    0x05, 0x85,        //   Usage Page (Battery System)
    0x09, 0x65,        //   Usage (AbsoluteStateOfCharge)
    0x15, 0x00,        //   Logical Minimum (0)
    0x26, 0x64, 0x00,  //   Logical Maximum (100)
    0x75, 0x08,        //   Report Size (8 bits)
    0x95, 0x01,        //   Report Count (1)
    0x81, 0x02,        //   Input (Data,Var,Abs)
    0x09, 0x44,        //   Usage (Charging)
    0x15, 0x00,        //   Logical Minimum (0)
    0x25, 0x01,        //   Logical Maximum (1)
    0x75, 0x08,        //   Report Size (8 bits)
    0x95, 0x01,        //   Report Count (1)
    0x81, 0x02,        //   Input (Data,Var,Abs)
    0xc0,              // End Collection
];

#[repr(C, packed)]
struct UhidCreate2Req {
    name: [u8; 128],
    phys: [u8; 64],
    uniq: [u8; 64],
    rd_size: u16,
    bus: u16,
    vendor: u32,
    product: u32,
    version: u32,
    country: u32,
    rd_data: [u8; 4096],
}

#[repr(C, packed)]
struct UhidCreate2Event {
    event_type: u32,
    req: UhidCreate2Req,
}

#[repr(C, packed)]
struct UhidInput2Req {
    size: u16,
    data: [u8; 4096],
}

#[repr(C, packed)]
struct UhidInput2Event {
    event_type: u32,
    req: UhidInput2Req,
}

#[repr(C, packed)]
struct UhidGetReportReplyReq {
    id: u32,
    err: u16,
    size: u16,
    data: [u8; 4096],
}

#[repr(C, packed)]
struct UhidGetReportReplyEvent {
    event_type: u32,
    req: UhidGetReportReplyReq,
}

#[repr(C, packed)]
struct UhidDestroyEvent {
    event_type: u32,
}

/// Virtual HID device creating a kernel power supply through /dev/uhid.
pub struct UhidBatteryDevice {
    file: File,
    current_percentage: Arc<AtomicU8>,
    current_charging: Arc<AtomicBool>,
}

impl UhidBatteryDevice {
    /// Creates and registers a virtual HID device via `/dev/uhid`.
    pub fn create(device_name: &str, vendor_id: u32, product_id: u32) -> Result<Self, DriverError> {
        let path = Path::new(UHID_PATH);
        if !path.exists() {
            return Err(DriverError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                format!("{UHID_PATH} does not exist. Make sure the 'uhid' kernel module is loaded ('modprobe uhid')."),
            )));
        }

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| {
                if e.kind() == io::ErrorKind::PermissionDenied {
                    DriverError::Io(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        format!(
                            "Permission denied opening {UHID_PATH}. Run as root, via systemd, or add KERNEL==\"uhid\", MODE=\"0666\" to udev rules."
                        ),
                    ))
                } else {
                    DriverError::Io(e)
                }
            })?;

        let mut create_ev = UhidCreate2Event {
            event_type: UHID_CREATE2,
            req: UhidCreate2Req {
                name: [0u8; 128],
                phys: [0u8; 64],
                uniq: [0u8; 64],
                rd_size: BATTERY_REPORT_DESCRIPTOR.len() as u16,
                bus: BUS_USB,
                vendor: vendor_id,
                product: product_id,
                version: 0,
                country: 0,
                rd_data: [0u8; 4096],
            },
        };

        let name_bytes = device_name.as_bytes();
        let name_len = name_bytes.len().min(127);
        create_ev.req.name[..name_len].copy_from_slice(&name_bytes[..name_len]);

        let phys = b"uhid/attack-shark-r1";
        create_ev.req.phys[..phys.len()].copy_from_slice(phys);

        create_ev.req.rd_data[..BATTERY_REPORT_DESCRIPTOR.len()]
            .copy_from_slice(BATTERY_REPORT_DESCRIPTOR);

        let raw_slice = unsafe {
            std::slice::from_raw_parts(
                (&create_ev as *const UhidCreate2Event) as *const u8,
                std::mem::size_of::<UhidCreate2Event>(),
            )
        };
        file.write_all(raw_slice)
            .map_err(|e| DriverError::Io(io::Error::new(e.kind(), format!("Failed to create UHID device: {e}"))))?;

        let current_percentage = Arc::new(AtomicU8::new(100));
        let current_charging = Arc::new(AtomicBool::new(false));
        let pct_clone = Arc::clone(&current_percentage);
        let chg_clone = Arc::clone(&current_charging);
        let mut read_file = file.try_clone().map_err(DriverError::Io)?;
        let mut write_clone = file.try_clone().map_err(DriverError::Io)?;

        // Reply to kernel GET_REPORT requests with current battery capacity and state.
        std::thread::spawn(move || {
            let mut buf = [0u8; 4380];
            loop {
                match read_file.read(&mut buf) {
                    Ok(n) if n >= 10 => {
                        let ev_type = u32::from_ne_bytes(buf[..4].try_into().unwrap_or([0; 4]));
                        if ev_type == UHID_GET_REPORT {
                            let req_id = u32::from_ne_bytes(buf[4..8].try_into().unwrap_or([0; 4]));
                            let current_pct = pct_clone.load(Ordering::Relaxed);
                            let current_chg = chg_clone.load(Ordering::Relaxed);

                            let mut reply = UhidGetReportReplyEvent {
                                event_type: UHID_GET_REPORT_REPLY,
                                req: UhidGetReportReplyReq {
                                    id: req_id,
                                    err: 0,
                                    size: 4,
                                    data: [0u8; 4096],
                                },
                            };
                            reply.req.data[0] = 0x01; // Report ID 1
                            reply.req.data[1] = 0x00; // Button = 0
                            reply.req.data[2] = current_pct;
                            reply.req.data[3] = current_chg as u8;

                            let reply_slice = unsafe {
                                std::slice::from_raw_parts(
                                    (&reply as *const UhidGetReportReplyEvent) as *const u8,
                                    16,
                                )
                            };
                            let _ = write_clone.write_all(reply_slice);
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            file,
            current_percentage,
            current_charging,
        })
    }

    /// Sends battery percentage (0..=100) and charging status to the kernel.
    pub fn update_battery(&mut self, percentage: u8, is_charging: bool) -> Result<(), DriverError> {
        let pct = percentage.min(100);
        self.current_percentage.store(pct, Ordering::Relaxed);
        self.current_charging.store(is_charging, Ordering::Relaxed);

        let mut input_ev = UhidInput2Event {
            event_type: UHID_INPUT2,
            req: UhidInput2Req {
                size: 4,
                data: [0u8; 4096],
            },
        };

        input_ev.req.data[0] = 0x01; // Report ID 1
        input_ev.req.data[1] = 0x00; // Button = 0
        input_ev.req.data[2] = pct;  // Capacity (0-100)
        input_ev.req.data[3] = is_charging as u8;

        let raw_slice = unsafe {
            std::slice::from_raw_parts(
                (&input_ev as *const UhidInput2Event) as *const u8,
                10,
            )
        };

        self.file
            .write_all(raw_slice)
            .map_err(|e| DriverError::Io(io::Error::new(e.kind(), format!("Failed to send UHID battery report: {e}"))))?;

        Ok(())
    }
}

impl Drop for UhidBatteryDevice {
    fn drop(&mut self) {
        let destroy_ev = UhidDestroyEvent {
            event_type: UHID_DESTROY,
        };
        let raw_slice = unsafe {
            std::slice::from_raw_parts(
                (&destroy_ev as *const UhidDestroyEvent) as *const u8,
                std::mem::size_of::<UhidDestroyEvent>(),
            )
        };
        let _ = self.file.write_all(raw_slice);
    }
}
