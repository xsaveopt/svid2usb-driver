#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no SVID2USB232 (eb1a:8286) is connected")]
    NotFound,
    #[error("the device has no isochronous video interface")]
    NoVideoInterface,
    #[error("the device was disconnected")]
    Disconnected,
    #[error("short transfer on register {0:#06x}")]
    ShortTransfer(u16),
    #[error("{name} must be in {min}..={max}, got {value}")]
    OutOfRange {
        name: &'static str,
        value: i32,
        min: i32,
        max: i32,
    },
    #[error("out of memory allocating USB transfers")]
    NoMemory,
    #[error("USB: {0}")]
    Usb(#[from] rusb::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
