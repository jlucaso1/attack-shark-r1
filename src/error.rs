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
            DriverError::DeviceNotFound => write!(f, "Attack Shark R1 mouse not found (neither wireless 0x1d57:0xfa60 nor wired 0x1d57:0xfa61)"),
            DriverError::Usb(e) => write!(f, "USB error: {e}"),
            DriverError::ClaimInterface(e) => write!(f, "Failed to claim USB interface: {e}"),
            #[cfg(feature = "polling-rate")]
            DriverError::InvalidPollingRate(r) => write!(f, "Invalid polling rate: {r}Hz (supported: 125, 250, 500, 1000)"),
            #[cfg(feature = "dpi")]
            DriverError::InvalidDpi(d) => write!(f, "Invalid DPI value: {d} (must be 100..=18000 in multiples of 100)"),
            #[cfg(feature = "dpi")]
            DriverError::InvalidDpiStage(s) => write!(f, "Invalid DPI stage index: {s} (must be 1..=6)"),
            #[cfg(feature = "sleep")]
            DriverError::InvalidSleepTime(t) => write!(f, "Invalid sleep time: {t}s (must be 0.5s..=30.0s)"),
            #[cfg(feature = "sleep")]
            DriverError::InvalidDeepSleepTime(t) => write!(f, "Invalid deep sleep time: {t}min (must be 1min..=60min)"),
            #[cfg(feature = "sleep")]
            DriverError::InvalidKeyResponseTime(t) => write!(f, "Invalid key response time: {t}ms (must be an even number between 4ms and 50ms)"),
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
