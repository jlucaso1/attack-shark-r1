use thiserror::Error;

#[derive(Error, Debug)]
pub enum DriverError {
    #[error("Attack Shark R1 mouse not found (neither wireless 0x1d57:0xfa60 nor wired 0x1d57:0xfa61)")]
    DeviceNotFound,

    #[error("USB error: {0}")]
    Usb(#[from] rusb::Error),

    #[error("Failed to claim USB interface: {0}")]
    ClaimInterface(rusb::Error),

    #[error("Invalid polling rate: {0}Hz (supported: 125, 250, 500, 1000)")]
    InvalidPollingRate(u32),

    #[error("Invalid DPI value: {0} (must be between 100 and 18000, in multiples of 100)")]
    InvalidDpi(u32),

    #[error("Invalid DPI stage index: {0} (must be between 1 and 6)")]
    InvalidDpiStage(u8),

    #[error("Invalid sleep time: {0}s (must be between 0.5s and 30.0s)")]
    InvalidSleepTime(f64),

    #[error("Invalid deep sleep time: {0}min (must be between 1min and 60min)")]
    InvalidDeepSleepTime(u8),

    #[error("Invalid key response / debounce time: {0}ms (must be an even number between 4ms and 50ms)")]
    InvalidKeyResponseTime(u8),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Device protocol error: {0}")]
    Protocol(String),

    #[error("Timeout while waiting for device response")]
    Timeout,
}
