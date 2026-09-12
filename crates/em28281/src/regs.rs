use crate::{FRAME_WIDTH, Input, Standard};

pub(crate) const CHIP_ID: u16 = 0x0a;
const USBSUSP: u16 = 0x0c;
const AUDIOSRC: u16 = 0x0e;
const XCLK: u16 = 0x0f;
const I2C_CLK: u16 = 0x06;
const VINENABLE: u16 = 0x12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Op {
    Write(u16, u8),
    WriteBits(u16, u8, u8),
    WriteBlock(u16, [u8; 2]),
    Sleep(u64),
}

const COLOR_DEFAULTS: &[(u16, u8)] = &[
    (0x20, 0x10),
    (0x21, 0x00),
    (0x22, 0x10),
    (0x23, 0x00),
    (0x24, 0x00),
    (0x25, 0x00),
    (0x14, 0x20),
    (0x15, 0x20),
    (0x16, 0x20),
    (0x17, 0x20),
    (0x18, 0x00),
    (0x19, 0x00),
    (0x1a, 0x00),
];

const VMUX_COMMON: &[(u16, u8)] = &[
    (0x24, 0x00),
    (0x25, 0x02),
    (0x2e, 0x00),
    (0x7a0b, 0x00),
    (0xb6, 0x8f),
    (0xb8, 0x00),
    (0x7a1c, 0x1e),
    (0x7a1d, 0x99),
    (0x7a1e, 0x99),
    (0x7a1f, 0x9a),
    (0x7a20, 0x3d),
    (0x7a21, 0x3e),
    (0x7a29, 0x00),
    (0x7a2f, 0x52),
    (0x7a40, 0x05),
    (0x7a51, 0x00),
    (0x7ac1, 0x1b),
];

const VMUX_COMPOSITE: &[(u16, u8)] = &[(0x38, 0x01), (0xb1, 0x70), (0xb3, 0x00), (0xb5, 0x00), (0x7a02, 0x4f)];

const VMUX_SVIDEO: &[(u16, u8)] = &[(0x38, 0x00), (0xb1, 0x60), (0xb3, 0x10), (0xb5, 0x10), (0x7a02, 0x4e)];

const NTSC_COMMON: &[(u16, u8)] = &[
    (0x7a01, 0x0d),
    (0x7a04, 0xdd),
    (0x7a07, 0x60),
    (0x7a08, 0x7a),
    (0x7a09, 0x02),
    (0x7a0a, 0x7c),
    (0x7a0c, 0x8a),
    (0x7a0f, 0x1c),
    (0x7a18, 0x20),
    (0x7a19, 0x74),
    (0x7a1a, 0x5d),
    (0x7a1b, 0x17),
    (0x7a2e, 0x85),
    (0x7a31, 0x63),
    (0x7a82, 0x42),
    (0x7ac0, 0xd4),
];

const NTSC_COMPOSITE: &[(u16, u8)] = &[(0x7a00, 0x00), (0x7a03, 0x00), (0x7a30, 0x22), (0x7a80, 0x03)];

const NTSC_SVIDEO: &[(u16, u8)] = &[(0x7a00, 0x01), (0x7a03, 0x03), (0x7a30, 0x20), (0x7a80, 0x04)];

const PAL_COMMON: &[(u16, u8)] = &[
    (0x7a04, 0xdc),
    (0x7a0c, 0x67),
    (0x7a0f, 0x1c),
    (0x7a18, 0x28),
    (0x7a19, 0x32),
    (0x7a1a, 0xb9),
    (0x7a1b, 0x86),
    (0x7a31, 0xc3),
    (0x7a82, 0x52),
];

const PAL_COMPOSITE: &[(u16, u8)] = &[
    (0x7a00, 0x32),
    (0x7a01, 0x10),
    (0x7a03, 0x06),
    (0x7a07, 0x2f),
    (0x7a08, 0x77),
    (0x7a09, 0x0f),
    (0x7a0a, 0x8c),
    (0x7a20, 0x3d),
    (0x7a2e, 0x88),
    (0x7a30, 0x2c),
    (0x7a80, 0x07),
];

const PAL_SVIDEO: &[(u16, u8)] = &[
    (0x7a00, 0x33),
    (0x7a01, 0x04),
    (0x7a03, 0x04),
    (0x7a07, 0x20),
    (0x7a08, 0x6a),
    (0x7a09, 0x16),
    (0x7a0a, 0x80),
    (0x7a2e, 0x8a),
    (0x7a30, 0x26),
    (0x7a80, 0x08),
];

const DECODER_LATCH: &[(u16, u8)] = &[(0x7a3f, 0x01), (0x7a3f, 0x00)];

fn writes(ops: &mut Vec<Op>, table: &[(u16, u8)]) {
    ops.extend(table.iter().map(|&(reg, val)| Op::Write(reg, val)));
}

pub(crate) fn init_ops() -> Vec<Op> {
    let mut ops = vec![Op::Write(XCLK, 0x07), Op::Write(I2C_CLK, 0x41), Op::Sleep(50)];
    writes(&mut ops, COLOR_DEFAULTS);
    ops
}

fn resolution_ops(ops: &mut Vec<Op>, standard: Standard) {
    let height = standard.height();
    let width = FRAME_WIDTH;
    ops.extend([
        Op::Write(0x27, 0x34),
        Op::Write(0x10, 0x10),
        Op::Write(0x34, 0x00),
        Op::Write(0x36, (width / 4) as u8),
        Op::Write(0x37, standard.vbi_lines() as u8),
        Op::Write(0x35, if standard == Standard::Ntsc { 0x09 } else { 0x07 }),
        Op::Write(0x11, 0x51),
        Op::Write(0x28, 0x01),
        Op::Write(0x29, ((width - 4) >> 2) as u8),
        Op::Write(0x2a, 0x01),
        Op::Write(0x2b, ((height - 4) >> 2) as u8),
        Op::Write(0x1c, 0x00),
        Op::Write(0x1d, 0x02),
        Op::Write(0x1e, (width >> 2) as u8),
        Op::Write(0x1f, (height >> 2) as u8),
        Op::Write(0x1b, (((height >> 9) & 0x02) | ((width >> 10) & 0x01)) as u8),
        Op::WriteBlock(0x30, [0, 0]),
        Op::WriteBlock(0x32, [0, 0]),
        Op::Write(0x26, 0x00),
    ]);
}

pub(crate) fn configure_ops(input: Input, standard: Standard) -> Vec<Op> {
    let mut ops = Vec::new();
    resolution_ops(&mut ops, standard);
    writes(&mut ops, VMUX_COMMON);
    writes(
        &mut ops,
        match input {
            Input::Composite => VMUX_COMPOSITE,
            Input::SVideo => VMUX_SVIDEO,
        },
    );
    writes(&mut ops, DECODER_LATCH);
    let (common, per_input) = match (standard, input) {
        (Standard::Ntsc, Input::Composite) => (NTSC_COMMON, NTSC_COMPOSITE),
        (Standard::Ntsc, Input::SVideo) => (NTSC_COMMON, NTSC_SVIDEO),
        (Standard::Pal, Input::Composite) => (PAL_COMMON, PAL_COMPOSITE),
        (Standard::Pal, Input::SVideo) => (PAL_COMMON, PAL_SVIDEO),
    };
    writes(&mut ops, common);
    writes(&mut ops, per_input);
    writes(&mut ops, DECODER_LATCH);
    ops.extend([
        Op::Write(XCLK, 0x87),
        Op::Sleep(10),
        Op::WriteBits(AUDIOSRC, 0x80, 0xc0),
        Op::Sleep(10),
    ]);
    resolution_ops(&mut ops, standard);
    ops
}

pub(crate) fn capture_ops(on: bool) -> Vec<Op> {
    if on {
        vec![
            Op::WriteBits(USBSUSP, 0x10, 0x10),
            Op::Write(0x48, 0x00),
            Op::Write(VINENABLE, 0x67),
            Op::Sleep(10),
        ]
    } else {
        vec![Op::WriteBits(USBSUSP, 0x00, 0x10), Op::Write(VINENABLE, 0x27)]
    }
}

pub(crate) fn last_write(reg: u16, input: Input, standard: Standard) -> Option<u8> {
    init_ops()
        .into_iter()
        .chain(configure_ops(input, standard))
        .filter_map(|op| match op {
            Op::Write(r, v) if r == reg => Some(v),
            _ => None,
        })
        .next_back()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntsc_capture_window_matches_linux() {
        let ops = configure_ops(Input::Composite, Standard::Ntsc);
        assert!(ops.contains(&Op::Write(0x2b, 119)));
        assert!(ops.contains(&Op::Write(0x1f, 120)));
        assert!(ops.contains(&Op::Write(0x37, 12)));
        assert!(ops.contains(&Op::Write(0x35, 0x09)));
    }

    #[test]
    fn pal_capture_window_matches_linux() {
        let ops = configure_ops(Input::SVideo, Standard::Pal);
        assert!(ops.contains(&Op::Write(0x2b, 143)));
        assert!(ops.contains(&Op::Write(0x1f, 144)));
        assert!(ops.contains(&Op::Write(0x37, 18)));
        assert!(ops.contains(&Op::Write(0x7a02, 0x4e)));
    }

    #[test]
    fn every_standard_ends_with_a_decoder_latch() {
        for standard in [Standard::Ntsc, Standard::Pal] {
            for input in [Input::Composite, Input::SVideo] {
                let ops = configure_ops(input, standard);
                let latch = ops.iter().rposition(|op| *op == Op::Write(0x7a3f, 0x00)).unwrap();
                assert_eq!(ops[latch - 1], Op::Write(0x7a3f, 0x01));
            }
        }
    }
}
