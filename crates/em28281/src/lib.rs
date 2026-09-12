mod controls;
mod device;
mod error;
mod frame;
mod regs;
mod stream;

pub use controls::{CONTROLS, Control};
pub use device::{Device, FrameSink};
pub use error::{Error, Result};
pub use frame::FrameAssembler;

pub const VENDOR_ID: u16 = 0xeb1a;
pub const PRODUCT_ID: u16 = 0x8286;
pub const FRAME_WIDTH: usize = 720;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Input {
    #[default]
    Composite,
    SVideo,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Standard {
    #[default]
    Ntsc,
    Pal,
}

impl Standard {
    #[must_use]
    pub fn height(self) -> usize {
        match self {
            Standard::Ntsc => 480,
            Standard::Pal => 576,
        }
    }

    pub(crate) fn vbi_lines(self) -> usize {
        match self {
            Standard::Ntsc => 12,
            Standard::Pal => 18,
        }
    }
}
