use crate::error::{Error, Result};
use crate::regs;
use crate::{Input, Standard};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Control {
    pub name: &'static str,
    pub label: &'static str,
    pub reg: u16,
    pub min: i32,
    pub max: i32,
    mask: u8,
}

const fn control(name: &'static str, label: &'static str, reg: u16, min: i32, max: i32) -> Control {
    Control {
        name,
        label,
        reg,
        min,
        max,
        mask: 0xff,
    }
}

pub const CONTROLS: &[Control] = &[
    control("brightness", "Brightness", 0x7a09, 0, 255),
    control("contrast", "Contrast", 0x7a08, 0, 255),
    control("saturation", "Saturation", 0x7a0a, 0, 255),
    control("hue", "Hue", 0x7a0b, -128, 127),
    Control {
        name: "sharpness",
        label: "Sharpness",
        reg: 0x25,
        min: 0,
        max: 15,
        mask: 0x0f,
    },
    control("luma_gain", "Luma gain", 0x20, 0, 31),
    control("luma_offset", "Luma offset", 0x21, -128, 127),
    control("chroma_gain", "Chroma gain", 0x22, 0, 31),
    control("blue_balance", "Blue balance", 0x23, -48, 48),
    control("red_balance", "Red balance", 0x24, -48, 48),
    control("gamma", "Gamma", 0x14, 0, 255),
    control("red_gain", "Red gain", 0x15, 0, 255),
    control("green_gain", "Green gain", 0x16, 0, 255),
    control("blue_gain", "Blue gain", 0x17, 0, 255),
    control("red_offset", "Red offset", 0x18, -128, 127),
    control("green_offset", "Green offset", 0x19, -128, 127),
    control("blue_offset", "Blue offset", 0x1a, -128, 127),
];

impl Control {
    pub fn check(&self, value: i32) -> Result<i32> {
        if (self.min..=self.max).contains(&value) {
            Ok(value)
        } else {
            Err(Error::OutOfRange {
                name: self.name,
                value,
                min: self.min,
                max: self.max,
            })
        }
    }

    pub(crate) fn mask(&self) -> u8 {
        self.mask
    }

    pub(crate) fn encode(&self, value: i32) -> u8 {
        (value as u8) & self.mask
    }

    pub(crate) fn decode(&self, raw: u8) -> i32 {
        let raw = raw & self.mask;
        if self.min < 0 {
            i32::from(raw as i8)
        } else {
            i32::from(raw)
        }
    }

    #[must_use]
    pub fn default_value(&self, input: Input, standard: Standard) -> i32 {
        self.decode(regs::last_write(self.reg, input, standard).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(name: &str) -> &'static Control {
        CONTROLS.iter().find(|c| c.name == name).unwrap()
    }

    #[test]
    fn every_control_has_a_default_from_the_init_sequence() {
        for standard in [Standard::Ntsc, Standard::Pal] {
            for input in [Input::Composite, Input::SVideo] {
                for c in CONTROLS {
                    assert!(
                        regs::last_write(c.reg, input, standard).is_some(),
                        "{} has no default",
                        c.name
                    );
                    let d = c.default_value(input, standard);
                    assert!(c.check(d).is_ok(), "{} default {d} out of range", c.name);
                }
            }
        }
    }

    #[test]
    fn defaults_follow_the_selected_standard() {
        let brightness = find("brightness");
        assert_eq!(brightness.default_value(Input::Composite, Standard::Ntsc), 0x02);
        assert_eq!(brightness.default_value(Input::Composite, Standard::Pal), 0x0f);
        assert_eq!(brightness.default_value(Input::SVideo, Standard::Pal), 0x16);
        assert_eq!(find("sharpness").default_value(Input::SVideo, Standard::Ntsc), 2);
    }

    #[test]
    fn signed_controls_round_trip() {
        let hue = find("hue");
        for v in [-128, -1, 0, 1, 127] {
            assert_eq!(hue.decode(hue.encode(v)), v);
        }
        assert_eq!(find("sharpness").decode(0xf7), 7);
    }

    #[test]
    fn out_of_range_is_rejected() {
        assert!(find("blue_balance").check(49).is_err());
    }
}
