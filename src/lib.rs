pub mod config;
pub mod dpi;
pub mod driver;
pub mod error;
pub mod protocol;
pub mod uhid;

pub use config::MouseConfig;
pub use dpi::{dpi_to_byte, DPI_LOOKUP_TABLE};
pub use driver::{AttackSharkR1, BatteryStatus};
pub use error::DriverError;
pub use protocol::{PollingRate, UsbTransport};
pub use uhid::UhidBatteryDevice;
