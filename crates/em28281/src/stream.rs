use std::ffi::{c_int, c_uint};
use std::time::{Duration, Instant};
use std::{mem, ptr, slice};

use libusb1_sys as ffi;
use libusb1_sys::constants::{
    LIBUSB_ERROR_BUSY, LIBUSB_ERROR_INVALID_PARAM, LIBUSB_ERROR_NO_DEVICE, LIBUSB_ERROR_NO_MEM,
    LIBUSB_ERROR_NOT_SUPPORTED, LIBUSB_TRANSFER_COMPLETED, LIBUSB_TRANSFER_ERROR, LIBUSB_TRANSFER_NO_DEVICE,
};
use rusb::{Context, DeviceHandle, UsbContext};

use crate::Standard;
use crate::device::FrameSink;
use crate::error::{Error, Result};
use crate::frame::FrameAssembler;

const NUM_TRANSFERS: usize = 8;
const PACKETS_PER_TRANSFER: usize = 64;
const DRAIN_TIMEOUT: Duration = Duration::from_secs(5);

struct Shared {
    assembler: FrameAssembler,
    sink: FrameSink,
    packet_size: usize,
    active: usize,
    stopping: bool,
    broken: bool,
}

impl Shared {
    fn deliver(&mut self, packets: &[ffi::libusb_iso_packet_descriptor], buffer: &[u8]) {
        let (width, height) = (self.assembler.width(), self.assembler.height());
        let Shared {
            assembler,
            sink,
            packet_size,
            ..
        } = self;
        for (packet, chunk) in packets.iter().zip(buffer.chunks(*packet_size)) {
            let len = packet.actual_length as usize;
            if packet.status == LIBUSB_TRANSFER_COMPLETED && len > 0 && len <= chunk.len() {
                assembler.push(&chunk[..len], &mut |frame| sink(frame, width, height));
            }
        }
    }

    fn complete(&mut self, status: c_int, resubmit: impl FnOnce() -> bool) {
        if status == LIBUSB_TRANSFER_NO_DEVICE {
            self.broken = true;
        }
        if !self.stopping && !self.broken {
            if resubmit() {
                return;
            }
            self.broken = true;
        }
        self.active -= 1;
    }
}

const fn transfer_length(packet_size: usize) -> usize {
    packet_size * PACKETS_PER_TRANSFER
}

const fn buffer_length(packet_size: usize) -> usize {
    transfer_length(packet_size) * NUM_TRANSFERS
}

pub(crate) struct Stream {
    context: Context,
    transfers: Vec<*mut ffi::libusb_transfer>,
    buffer: Box<[u8]>,
    shared: *mut Shared,
}

unsafe impl Send for Stream {}

impl Stream {
    pub(crate) fn new(
        handle: &DeviceHandle<Context>,
        endpoint: u8,
        packet_size: usize,
        standard: Standard,
        sink: FrameSink,
    ) -> Result<Self> {
        let shared = Box::into_raw(Box::new(Shared {
            assembler: FrameAssembler::new(standard),
            sink,
            packet_size,
            active: 0,
            stopping: false,
            broken: false,
        }));
        let length = transfer_length(packet_size);
        let mut stream = Stream {
            context: handle.context().clone(),
            transfers: Vec::with_capacity(NUM_TRANSFERS),
            buffer: vec![0u8; buffer_length(packet_size)].into_boxed_slice(),
            shared,
        };
        for chunk in stream.buffer.chunks_exact_mut(length) {
            let transfer = unsafe {
                let transfer = ffi::libusb_alloc_transfer(PACKETS_PER_TRANSFER as c_int);
                if !transfer.is_null() {
                    ffi::libusb_fill_iso_transfer(
                        transfer,
                        handle.as_raw(),
                        endpoint,
                        chunk.as_mut_ptr(),
                        length as c_int,
                        PACKETS_PER_TRANSFER as c_int,
                        on_transfer,
                        shared.cast(),
                        0,
                    );
                    ffi::libusb_set_iso_packet_lengths(transfer, packet_size as c_uint);
                }
                transfer
            };
            if transfer.is_null() {
                return Err(Error::NoMemory);
            }
            stream.transfers.push(transfer);
        }
        Ok(stream)
    }

    fn shared(&mut self) -> &mut Shared {
        unsafe { &mut *self.shared }
    }

    pub(crate) fn submit(&mut self) -> Result<()> {
        let mut submitted = 0;
        let mut result = Ok(());
        for &transfer in &self.transfers {
            let rc = unsafe { ffi::libusb_submit_transfer(transfer) };
            if rc < 0 {
                result = Err(usb_error(rc));
                break;
            }
            submitted += 1;
        }
        self.shared().active += submitted;
        result
    }

    pub(crate) fn poll(&mut self, timeout: Duration) -> Result<()> {
        match self.context.handle_events(Some(timeout)) {
            Ok(()) | Err(rusb::Error::Interrupted) => {}
            Err(e) => return Err(e.into()),
        }
        if self.shared().broken {
            Err(Error::Disconnected)
        } else {
            Ok(())
        }
    }

    fn drain(&mut self) -> bool {
        self.shared().stopping = true;
        for &transfer in &self.transfers {
            unsafe { ffi::libusb_cancel_transfer(transfer) };
        }
        let deadline = Instant::now() + DRAIN_TIMEOUT;
        while self.shared().active > 0 && Instant::now() < deadline {
            let _ = self.context.handle_events(Some(Duration::from_millis(100)));
        }
        self.shared().active == 0
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        if !self.drain() {
            mem::forget(mem::take(&mut self.buffer));
            return;
        }
        unsafe {
            for &transfer in &self.transfers {
                ffi::libusb_free_transfer(transfer);
            }
            drop(Box::from_raw(self.shared));
        }
    }
}

extern "system" fn on_transfer(transfer: *mut ffi::libusb_transfer) {
    let (shared, status, packets, buffer) = unsafe {
        let packets = ptr::addr_of!((*transfer).iso_packet_desc).cast::<ffi::libusb_iso_packet_descriptor>();
        (
            &mut *(*transfer).user_data.cast::<Shared>(),
            (*transfer).status,
            slice::from_raw_parts(packets, (*transfer).num_iso_packets as usize),
            slice::from_raw_parts((*transfer).buffer, (*transfer).length as usize),
        )
    };
    if status == LIBUSB_TRANSFER_COMPLETED || status == LIBUSB_TRANSFER_ERROR {
        shared.deliver(packets, buffer);
    }
    shared.complete(status, || unsafe { ffi::libusb_submit_transfer(transfer) } == 0);
}

fn usb_error(code: c_int) -> Error {
    Error::Usb(match code {
        LIBUSB_ERROR_NO_DEVICE => rusb::Error::NoDevice,
        LIBUSB_ERROR_BUSY => rusb::Error::Busy,
        LIBUSB_ERROR_NO_MEM => rusb::Error::NoMem,
        LIBUSB_ERROR_INVALID_PARAM => rusb::Error::InvalidParam,
        LIBUSB_ERROR_NOT_SUPPORTED => rusb::Error::NotSupported,
        _ => rusb::Error::Other,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use libusb1_sys::constants::LIBUSB_TRANSFER_TIMED_OUT;

    use super::*;
    use crate::FRAME_WIDTH;

    type Frames = Arc<Mutex<Vec<(usize, usize, usize)>>>;

    const FIELD_HEADER: [u8; 4] = [0x22, 0x5a, 0, 0];

    fn shared(packet_size: usize) -> (Shared, Frames) {
        let frames: Frames = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&frames);
        let shared = Shared {
            assembler: FrameAssembler::new(Standard::Ntsc),
            sink: Box::new(move |frame: &[u8], width, height| {
                recorded.lock().unwrap().push((frame.len(), width, height));
            }),
            packet_size,
            active: 0,
            stopping: false,
            broken: false,
        };
        (shared, frames)
    }

    fn packet(status: c_int, actual_length: u32) -> ffi::libusb_iso_packet_descriptor {
        ffi::libusb_iso_packet_descriptor {
            length: 0,
            actual_length,
            status,
        }
    }

    fn headers(count: usize) -> Vec<u8> {
        FIELD_HEADER.repeat(count)
    }

    fn frames_of(frames: &Frames) -> Vec<(usize, usize, usize)> {
        frames.lock().unwrap().clone()
    }

    #[test]
    fn the_buffer_holds_one_whole_chunk_per_transfer() {
        assert_eq!(transfer_length(3072), 3072 * PACKETS_PER_TRANSFER);
        assert_eq!(buffer_length(3072), 3072 * PACKETS_PER_TRANSFER * NUM_TRANSFERS);

        let chunk = transfer_length(2888);
        let buffer = vec![0u8; buffer_length(2888)];
        assert_eq!(buffer.len() % chunk, 0);
        assert_eq!(buffer.len() / chunk, NUM_TRANSFERS);
    }

    #[test]
    fn every_packet_of_a_transfer_reaches_the_assembler() {
        let (mut shared, frames) = shared(4);
        let descriptors = [
            packet(LIBUSB_TRANSFER_COMPLETED, 4),
            packet(LIBUSB_TRANSFER_COMPLETED, 4),
        ];
        shared.deliver(&descriptors, &headers(2));
        assert_eq!(frames_of(&frames), vec![(FRAME_WIDTH * 2 * 480, FRAME_WIDTH, 480)]);
    }

    #[test]
    fn failed_empty_and_overlong_packets_are_dropped() {
        let (mut shared, frames) = shared(4);
        let descriptors = [
            packet(LIBUSB_TRANSFER_ERROR, 4),
            packet(LIBUSB_TRANSFER_COMPLETED, 0),
            packet(LIBUSB_TRANSFER_COMPLETED, 5),
            packet(LIBUSB_TRANSFER_TIMED_OUT, 4),
        ];
        shared.deliver(&descriptors, &headers(4));
        assert!(frames_of(&frames).is_empty());
    }

    #[test]
    fn a_packet_is_clipped_to_its_own_chunk() {
        let (mut shared, frames) = shared(4);
        let mut buffer = headers(1);
        buffer.extend_from_slice(&FIELD_HEADER[..2]);
        let descriptors = [
            packet(LIBUSB_TRANSFER_COMPLETED, 4),
            packet(LIBUSB_TRANSFER_COMPLETED, 4),
        ];
        shared.deliver(&descriptors, &buffer);
        assert!(frames_of(&frames).is_empty());
    }

    #[test]
    fn packets_past_the_end_of_the_buffer_are_ignored() {
        let (mut shared, frames) = shared(4);
        let descriptors: [_; 4] = std::array::from_fn(|_| packet(LIBUSB_TRANSFER_COMPLETED, 4));
        shared.deliver(&descriptors, &headers(2));
        assert_eq!(frames_of(&frames).len(), 1);
    }

    #[test]
    fn a_resubmitted_transfer_stays_active() {
        let (mut shared, _frames) = shared(4);
        shared.active = NUM_TRANSFERS;
        let mut tried = false;
        shared.complete(LIBUSB_TRANSFER_COMPLETED, || {
            tried = true;
            true
        });
        assert!(tried);
        assert_eq!(shared.active, NUM_TRANSFERS);
        assert!(!shared.broken);
    }

    #[test]
    fn a_refused_resubmit_breaks_the_stream() {
        let (mut shared, _frames) = shared(4);
        shared.active = NUM_TRANSFERS;
        shared.complete(LIBUSB_TRANSFER_ERROR, || false);
        assert_eq!(shared.active, NUM_TRANSFERS - 1);
        assert!(shared.broken);
    }

    #[test]
    fn a_lost_device_breaks_the_stream_without_resubmitting() {
        let (mut shared, _frames) = shared(4);
        shared.active = NUM_TRANSFERS;
        shared.complete(LIBUSB_TRANSFER_NO_DEVICE, || {
            panic!("a lost device must not be resubmitted")
        });
        assert_eq!(shared.active, NUM_TRANSFERS - 1);
        assert!(shared.broken);
    }

    #[test]
    fn a_broken_stream_retires_the_transfers_it_has_left() {
        let (mut shared, _frames) = shared(4);
        shared.active = NUM_TRANSFERS;
        shared.broken = true;
        for _ in 0..NUM_TRANSFERS {
            shared.complete(LIBUSB_TRANSFER_COMPLETED, || {
                panic!("a broken stream must not be resubmitted")
            });
        }
        assert_eq!(shared.active, 0);
    }

    #[test]
    fn stopping_retires_every_transfer_and_is_not_a_failure() {
        let (mut shared, _frames) = shared(4);
        shared.active = NUM_TRANSFERS;
        shared.stopping = true;
        for _ in 0..NUM_TRANSFERS {
            shared.complete(LIBUSB_TRANSFER_COMPLETED, || {
                panic!("a stopping stream must not be resubmitted")
            });
        }
        assert_eq!(shared.active, 0);
        assert!(!shared.broken);
    }

    #[test]
    fn libusb_codes_map_onto_rusb_errors() {
        assert!(matches!(
            usb_error(LIBUSB_ERROR_NO_DEVICE),
            Error::Usb(rusb::Error::NoDevice)
        ));
        assert!(matches!(usb_error(LIBUSB_ERROR_BUSY), Error::Usb(rusb::Error::Busy)));
        assert!(matches!(usb_error(LIBUSB_ERROR_NO_MEM), Error::Usb(rusb::Error::NoMem)));
        assert!(matches!(
            usb_error(LIBUSB_ERROR_INVALID_PARAM),
            Error::Usb(rusb::Error::InvalidParam)
        ));
        assert!(matches!(
            usb_error(LIBUSB_ERROR_NOT_SUPPORTED),
            Error::Usb(rusb::Error::NotSupported)
        ));
        assert!(matches!(usb_error(-99), Error::Usb(rusb::Error::Other)));
    }
}
