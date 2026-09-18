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
pub use dpi::{dpi_to_byte, validate_dpi, DPI_LOOKUP_TABLE, DPI_MAX, DPI_MIN, DPI_STEP};
pub use driver::{AttackSharkR1, BatteryStatus};
pub use error::DriverError;
#[cfg(feature = "polling-rate")]
pub use protocol::PollingRate;
pub use protocol::{
    UsbTransport, INTERFACE_NUMBER, PRODUCT_ID_WIRED, PRODUCT_ID_WIRELESS, VENDOR_ID,
};
pub use uhid::{UhidBatteryDevice, DEVICE_NAME};

