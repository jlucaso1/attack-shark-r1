use std::fmt;

#[derive(Debug)]
pub enum DriverError {
    DeviceNotFound,
    Usb(rusb::Error),
    ClaimInterface(rusb::Error),
    #[cfg(feature = "polling-rate")]
    InvalidPollingRate(u32),
    #[cfg(feature = "dpi")]
    InvalidDpi(u32),
    #[cfg(feature = "dpi")]
    InvalidDpiStage(u8),
    #[cfg(feature = "sleep")]
    InvalidSleepTime(f64),
    #[cfg(feature = "sleep")]
    InvalidDeepSleepTime(u8),
    #[cfg(feature = "sleep")]
    InvalidKeyResponseTime(u8),
    #[cfg(feature = "config")]
    Config(String),
    Io(std::io::Error),
    Protocol(String),
    Timeout,
}

impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DriverError::DeviceNotFound => write!(
                f,
                "{} mouse not found (neither wireless 0x{:04x}:0x{:04x} nor wired 0x{:04x}:0x{:04x})",
                crate::uhid::DEVICE_NAME,
                crate::protocol::VENDOR_ID,
                crate::protocol::PRODUCT_ID_WIRELESS,
                crate::protocol::VENDOR_ID,
                crate::protocol::PRODUCT_ID_WIRED,
            ),
            DriverError::Usb(e) => write!(f, "USB error: {e}"),
            DriverError::ClaimInterface(e) => write!(f, "Failed to claim USB interface: {e}"),
            #[cfg(feature = "polling-rate")]
            DriverError::InvalidPollingRate(r) => write!(
                f,
                "Invalid polling rate: {r}Hz (supported: {}, {}, {}, {})",
                crate::protocol::POLLING_RATE_125_HZ,
                crate::protocol::POLLING_RATE_250_HZ,
                crate::protocol::POLLING_RATE_500_HZ,
                crate::protocol::POLLING_RATE_1000_HZ,
            ),
            #[cfg(feature = "dpi")]
            DriverError::InvalidDpi(d) => write!(
                f,
                "Invalid DPI value: {d} (must be {}..={} in multiples of {})",
                crate::dpi::DPI_MIN,
                crate::dpi::DPI_MAX,
                crate::dpi::DPI_STEP,
            ),
            #[cfg(feature = "dpi")]
            DriverError::InvalidDpiStage(s) => write!(
                f,
                "Invalid DPI stage index: {s} (must be {}..={})",
                crate::dpi::DPI_STAGE_MIN,
                crate::dpi::DPI_STAGE_MAX,
            ),
            #[cfg(feature = "sleep")]
            DriverError::InvalidSleepTime(t) => write!(
                f,
                "Invalid sleep time: {t}s (must be {}s..={}s)",
                crate::driver::SLEEP_TIME_MIN,
                crate::driver::SLEEP_TIME_MAX,
            ),
            #[cfg(feature = "sleep")]
            DriverError::InvalidDeepSleepTime(t) => write!(
                f,
                "Invalid deep sleep time: {t}min (must be {}min..={}min)",
                crate::driver::DEEP_SLEEP_MIN,
                crate::driver::DEEP_SLEEP_MAX,
            ),
            #[cfg(feature = "sleep")]
            DriverError::InvalidKeyResponseTime(t) => write!(
                f,
                "Invalid key response time: {t}ms (must be an even number between {}ms and {}ms)",
                crate::driver::DEBOUNCE_MIN,
                crate::driver::DEBOUNCE_MAX,
            ),
            #[cfg(feature = "config")]
            DriverError::Config(msg) => write!(f, "Configuration error: {msg}"),
            DriverError::Io(e) => write!(f, "IO error: {e}"),
            DriverError::Protocol(msg) => write!(f, "Device protocol error: {msg}"),
            DriverError::Timeout => write!(f, "Timeout waiting for device response"),
        }
    }
}

impl std::error::Error for DriverError {}

impl From<rusb::Error> for DriverError {
    fn from(e: rusb::Error) -> Self {
        DriverError::Usb(e)
    }
}

impl From<std::io::Error> for DriverError {
    fn from(e: std::io::Error) -> Self {
        DriverError::Io(e)
    }
}
