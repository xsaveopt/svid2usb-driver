use std::time::Duration;

use rusb::{Context, DeviceHandle, TransferType, UsbContext};

use crate::controls::Control;
use crate::error::{Error, Result};
use crate::regs::{self, Op};
use crate::stream::Stream;
use crate::{FRAME_WIDTH, Input, PRODUCT_ID, Standard, VENDOR_ID};

const TIMEOUT: Duration = Duration::from_secs(1);
const VIDEO_ENDPOINT: u8 = 0x82;
const WANTED_PACKET_SIZE: usize = (FRAME_WIDTH * 2 + 4) * 2;
const VENDOR_IN: u8 = 0xc0;
const VENDOR_OUT: u8 = 0x40;

pub type FrameSink = Box<dyn FnMut(&[u8], usize, usize) + Send>;

struct VideoInterface {
    number: u8,
    alternates: Vec<(u8, usize)>,
}

pub struct Device {
    stream: Option<Stream>,
    video: VideoInterface,
    handle: DeviceHandle<Context>,
    standard: Standard,
}

impl Device {
    pub fn open() -> Result<Self> {
        let handle = open_handle()?;
        let video = find_video_interface(&handle)?;
        handle.claim_interface(video.number)?;
        Ok(Self {
            stream: None,
            video,
            handle,
            standard: Standard::default(),
        })
    }

    pub fn read_register(&self, reg: u16) -> Result<u8> {
        let mut buf = [0u8; 1];
        match self.handle.read_control(VENDOR_IN, 0, 0, reg, &mut buf, TIMEOUT)? {
            1 => Ok(buf[0]),
            _ => Err(Error::ShortTransfer(reg)),
        }
    }

    pub fn write_register(&self, reg: u16, value: u8) -> Result<()> {
        self.write_block(reg, &[value])
    }

    fn write_block(&self, reg: u16, data: &[u8]) -> Result<()> {
        if self.handle.write_control(VENDOR_OUT, 0, 0, reg, data, TIMEOUT)? == data.len() {
            Ok(())
        } else {
            Err(Error::ShortTransfer(reg))
        }
    }

    fn write_bits(&self, reg: u16, value: u8, mask: u8) -> Result<()> {
        let old = self.read_register(reg)?;
        self.write_register(reg, (old & !mask) | (value & mask))
    }

    fn run(&self, ops: &[Op]) -> Result<()> {
        for op in ops {
            match *op {
                Op::Write(reg, value) => self.write_register(reg, value)?,
                Op::WriteBits(reg, value, mask) => self.write_bits(reg, value, mask)?,
                Op::WriteBlock(reg, data) => self.write_block(reg, &data)?,
                Op::Sleep(ms) => std::thread::sleep(Duration::from_millis(ms)),
            }
        }
        Ok(())
    }

    pub fn chip_id(&self) -> Result<u8> {
        self.read_register(regs::CHIP_ID)
    }

    pub fn init(&self) -> Result<()> {
        self.chip_id()?;
        self.run(&regs::init_ops())
    }

    pub fn configure(&mut self, input: Input, standard: Standard) -> Result<()> {
        self.stop();
        self.run(&regs::configure_ops(input, standard))?;
        self.standard = standard;
        Ok(())
    }

    pub fn set(&self, control: &Control, value: i32) -> Result<()> {
        let raw = control.encode(control.check(value)?);
        if control.mask() == 0xff {
            self.write_register(control.reg, raw)
        } else {
            self.write_bits(control.reg, raw, control.mask())
        }
    }

    pub fn start(&mut self, sink: FrameSink) -> Result<()> {
        self.stop();
        let video = &self.video;
        let (alternate, packet_size) =
            pick_alternate(&video.alternates, WANTED_PACKET_SIZE).ok_or(Error::NoVideoInterface)?;
        self.handle.set_alternate_setting(video.number, alternate)?;

        let mut stream = Stream::new(&self.handle, VIDEO_ENDPOINT, packet_size, self.standard, sink)?;
        let started = self.run(&regs::capture_ops(true)).and_then(|()| stream.submit());
        self.stream = Some(stream);
        if let Err(e) = started {
            self.stop();
            return Err(e);
        }
        Ok(())
    }

    pub fn poll(&mut self, timeout: Duration) -> Result<()> {
        if let Some(stream) = &mut self.stream {
            return stream.poll(timeout);
        }
        std::thread::sleep(timeout);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(stream) = self.stream.take() {
            drop(stream);
            let _ = self.run(&regs::capture_ops(false));
            let _ = self.handle.set_alternate_setting(self.video.number, 0);
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.stop();
    }
}

fn open_handle() -> Result<DeviceHandle<Context>> {
    let context = Context::new()?;
    for device in context.devices()?.iter() {
        let Ok(descriptor) = device.device_descriptor() else {
            continue;
        };
        if descriptor.vendor_id() == VENDOR_ID && descriptor.product_id() == PRODUCT_ID {
            return Ok(device.open()?);
        }
    }
    Err(Error::NotFound)
}

fn iso_packet_size(max_packet_size: u16) -> usize {
    usize::from(max_packet_size & 0x7ff) * (1 + usize::from((max_packet_size >> 11) & 0x03))
}

fn is_video_endpoint(address: u8, transfer_type: TransferType) -> bool {
    address == VIDEO_ENDPOINT && transfer_type == TransferType::Isochronous
}

fn pick_alternate(alternates: &[(u8, usize)], wanted: usize) -> Option<(u8, usize)> {
    alternates
        .iter()
        .find(|&&(_, size)| size >= wanted)
        .or_else(|| alternates.iter().max_by_key(|&&(_, size)| size))
        .copied()
}

fn find_video_interface(handle: &DeviceHandle<Context>) -> Result<VideoInterface> {
    let config = handle.device().active_config_descriptor()?;
    for interface in config.interfaces() {
        let alternates: Vec<(u8, usize)> = interface
            .descriptors()
            .filter_map(|setting| {
                setting
                    .endpoint_descriptors()
                    .find(|e| is_video_endpoint(e.address(), e.transfer_type()))
                    .map(|e| (setting.setting_number(), iso_packet_size(e.max_packet_size())))
            })
            .collect();
        if !alternates.is_empty() {
            return Ok(VideoInterface {
                number: interface.number(),
                alternates,
            });
        }
    }
    Err(Error::NoVideoInterface)
}

#[cfg(test)]
mod tests {
    use rusb::TransferType;

    use super::{VIDEO_ENDPOINT, WANTED_PACKET_SIZE, is_video_endpoint, iso_packet_size, pick_alternate};

    #[test]
    fn high_bandwidth_packet_sizes() {
        assert_eq!(iso_packet_size(0x1400), 3 * 1024);
        assert_eq!(iso_packet_size(0x0b20), 2 * 800);
        assert_eq!(iso_packet_size(0x0000), 0);
    }

    #[test]
    fn a_full_line_pair_plus_headers_is_wanted() {
        assert_eq!(WANTED_PACKET_SIZE, 2888);
    }

    #[test]
    fn only_the_isochronous_video_endpoint_matches() {
        assert!(is_video_endpoint(VIDEO_ENDPOINT, TransferType::Isochronous));
        assert!(!is_video_endpoint(VIDEO_ENDPOINT, TransferType::Bulk));
        assert!(!is_video_endpoint(VIDEO_ENDPOINT, TransferType::Interrupt));
        assert!(!is_video_endpoint(0x83, TransferType::Isochronous));
        assert!(!is_video_endpoint(VIDEO_ENDPOINT & 0x7f, TransferType::Isochronous));
    }

    #[test]
    fn the_first_alternate_that_fits_wins() {
        let alternates = [(0, 0), (1, 1024), (2, 2048), (3, 2888), (4, 3072)];
        assert_eq!(pick_alternate(&alternates, WANTED_PACKET_SIZE), Some((3, 2888)));
    }

    #[test]
    fn a_larger_earlier_alternate_is_not_traded_for_a_tighter_later_one() {
        let alternates = [(1, 3072), (2, 2888)];
        assert_eq!(pick_alternate(&alternates, WANTED_PACKET_SIZE), Some((1, 3072)));
    }

    #[test]
    fn the_largest_alternate_is_the_fallback() {
        let alternates = [(0, 0), (1, 1448), (2, 2048), (3, 1024)];
        assert_eq!(pick_alternate(&alternates, WANTED_PACKET_SIZE), Some((2, 2048)));
    }

    #[test]
    fn an_interface_without_alternates_has_nothing_to_pick() {
        assert_eq!(pick_alternate(&[], WANTED_PACKET_SIZE), None);
    }

    #[test]
    fn a_zero_sized_alternate_is_still_the_fallback() {
        assert_eq!(pick_alternate(&[(0, 0)], WANTED_PACKET_SIZE), Some((0, 0)));
    }
}
