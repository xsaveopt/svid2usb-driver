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

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use super::*;

    #[test]
    fn messages_name_the_failure() {
        assert_eq!(Error::NotFound.to_string(), "no SVID2USB232 (eb1a:8286) is connected");
        assert_eq!(
            Error::NoVideoInterface.to_string(),
            "the device has no isochronous video interface"
        );
        assert_eq!(Error::Disconnected.to_string(), "the device was disconnected");
        assert_eq!(Error::NoMemory.to_string(), "out of memory allocating USB transfers");
    }

    #[test]
    fn a_short_transfer_reports_the_register_in_hex() {
        assert_eq!(
            Error::ShortTransfer(0x7a09).to_string(),
            "short transfer on register 0x7a09"
        );
        assert_eq!(Error::ShortTransfer(0).to_string(), "short transfer on register 0x0000");
    }

    #[test]
    fn an_out_of_range_control_reports_its_bounds() {
        let error = Error::OutOfRange {
            name: "hue",
            value: 200,
            min: -128,
            max: 127,
        };
        assert_eq!(error.to_string(), "hue must be in -128..=127, got 200");
    }

    #[test]
    fn usb_failures_convert_and_keep_their_cause() {
        let error = Error::from(rusb::Error::NoDevice);
        assert!(matches!(error, Error::Usb(rusb::Error::NoDevice)));
        assert_eq!(error.to_string(), format!("USB: {}", rusb::Error::NoDevice));
        assert!(error.source().is_some());
        assert!(Error::NotFound.source().is_none());
    }

    #[test]
    fn the_question_mark_operator_widens_a_usb_failure() {
        fn widen(raw: std::result::Result<u8, rusb::Error>) -> Result<u8> {
            Ok(raw? + 1)
        }
        assert_eq!(widen(Ok(1)).unwrap(), 2);
        assert_eq!(
            widen(Err(rusb::Error::Busy)).unwrap_err().to_string(),
            format!("USB: {}", rusb::Error::Busy)
        );
    }
}
