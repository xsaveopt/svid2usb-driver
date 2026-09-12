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
        let length = packet_size * PACKETS_PER_TRANSFER;
        let mut stream = Stream {
            context: handle.context().clone(),
            transfers: Vec::with_capacity(NUM_TRANSFERS),
            buffer: vec![0u8; length * NUM_TRANSFERS].into_boxed_slice(),
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
        let Shared {
            assembler,
            sink,
            packet_size,
            ..
        } = shared;
        let (width, height) = (assembler.width(), assembler.height());
        for (packet, chunk) in packets.iter().zip(buffer.chunks(*packet_size)) {
            let len = packet.actual_length as usize;
            if packet.status == LIBUSB_TRANSFER_COMPLETED && len > 0 && len <= chunk.len() {
                assembler.push(&chunk[..len], &mut |frame| sink(frame, width, height));
            }
        }
    }
    if status == LIBUSB_TRANSFER_NO_DEVICE {
        shared.broken = true;
    }
    if !shared.stopping && !shared.broken {
        if unsafe { ffi::libusb_submit_transfer(transfer) } == 0 {
            return;
        }
        shared.broken = true;
    }
    shared.active -= 1;
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
