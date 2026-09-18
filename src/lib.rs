#[cfg(feature = "config")]
pub mod config;
#[cfg(feature = "dpi")]
pub mod dpi;
pub mod driver;
pub mod error;
pub mod protocol;
pub mod uhid;

#[cfg(feature = "config")]
pub use config::MouseConfig;
#[cfg(feature = "dpi")]
pub use dpi::{dpi_to_byte, DPI_LOOKUP_TABLE};
pub use driver::{AttackSharkR1, BatteryStatus};
pub use error::DriverError;
#[cfg(feature = "polling-rate")]
pub use protocol::PollingRate;
pub use protocol::UsbTransport;
pub use uhid::UhidBatteryDevice;
