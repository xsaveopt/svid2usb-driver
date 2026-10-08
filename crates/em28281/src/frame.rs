use crate::{FRAME_WIDTH, Standard};

const BYTES_PER_LINE: usize = FRAME_WIDTH * 2;
const BLACK_YUYV: [u8; 4] = [0x10, 0x80, 0x10, 0x80];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Idle,
    VbiStart,
    Vbi,
    VideoStart,
    Video,
}

pub struct FrameAssembler {
    height: usize,
    vbi_size: usize,
    frame: Vec<u8>,
    state: State,
    top_field: bool,
    writing_top: bool,
    have_frame: bool,
    pos: usize,
    vbi_read: usize,
}

impl FrameAssembler {
    #[must_use]
    pub fn new(standard: Standard) -> Self {
        let height = standard.height();
        let frame = BLACK_YUYV
            .iter()
            .copied()
            .cycle()
            .take(BYTES_PER_LINE * height)
            .collect();
        Self {
            height,
            vbi_size: FRAME_WIDTH * standard.vbi_lines(),
            frame,
            state: State::Idle,
            top_field: true,
            writing_top: true,
            have_frame: false,
            pos: 0,
            vbi_read: 0,
        }
    }

    #[must_use]
    pub fn width(&self) -> usize {
        FRAME_WIDTH
    }

    #[must_use]
    pub fn height(&self) -> usize {
        self.height
    }

    pub fn push(&mut self, packet: &[u8], emit: &mut dyn FnMut(&[u8])) {
        let mut data = packet;
        if data.len() >= 4 {
            match data[..4] {
                [0x88, 0x88, 0x88, 0x88] => data = &data[4..],
                [0x33, 0x95, field, _] => {
                    self.state = State::VbiStart;
                    self.vbi_read = 0;
                    self.top_field = field & 1 == 0;
                    data = &data[4..];
                }
                [0x22, 0x5a, field, _] => {
                    self.state = State::VideoStart;
                    self.top_field = field & 1 == 0;
                    data = &data[4..];
                }
                _ => {}
            }
        }

        if self.state == State::VbiStart {
            self.state = State::Vbi;
        }

        if self.state == State::Vbi {
            let n = data.len().min(self.vbi_size - self.vbi_read);
            self.vbi_read += n;
            if n < data.len() {
                self.state = State::VideoStart;
                data = &data[n..];
            }
        }

        if self.state == State::VideoStart {
            self.start_field(emit);
            self.state = State::Video;
        }

        if self.state == State::Video && !data.is_empty() {
            self.copy_video(data);
        }
    }

    fn start_field(&mut self, emit: &mut dyn FnMut(&[u8])) {
        if self.top_field {
            if self.have_frame {
                emit(&self.frame);
            }
            self.have_frame = true;
        }
        self.writing_top = self.top_field;
        self.pos = 0;
    }

    fn copy_video(&mut self, mut data: &[u8]) {
        let field_bytes = BYTES_PER_LINE * (self.height / 2);
        let room = field_bytes.saturating_sub(self.pos);
        if data.len() > room {
            data = &data[..room];
        }
        let field_offset = if self.writing_top { 0 } else { BYTES_PER_LINE };
        while !data.is_empty() {
            let line = self.pos / BYTES_PER_LINE;
            let col = self.pos % BYTES_PER_LINE;
            let n = (BYTES_PER_LINE - col).min(data.len());
            let at = line * 2 * BYTES_PER_LINE + field_offset + col;
            self.frame[at..at + n].copy_from_slice(&data[..n]);
            self.pos += n;
            data = &data[n..];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_value(top: bool, line: usize) -> u8 {
        (if top { 0x40 } else { 0x80 }) + (line % 64) as u8
    }

    fn feed_field(
        asm: &mut FrameAssembler,
        standard: Standard,
        top: bool,
        packet: usize,
        vbi: bool,
        emit: &mut dyn FnMut(&[u8]),
    ) {
        let vbi_bytes = if vbi { FRAME_WIDTH * standard.vbi_lines() } else { 0 };
        let mut stream = vec![0x11; vbi_bytes];
        for line in 0..standard.height() / 2 {
            stream.extend(std::iter::repeat_n(line_value(top, line), BYTES_PER_LINE));
        }
        let header = if vbi { [0x33, 0x95] } else { [0x22, 0x5a] };
        for (i, chunk) in stream.chunks(packet - 4).enumerate() {
            let mut buf = if i == 0 {
                vec![header[0], header[1], u8::from(!top), 0]
            } else {
                vec![0x88; 4]
            };
            buf.extend_from_slice(chunk);
            asm.push(&buf, emit);
        }
    }

    fn check_frame(frame: &[u8], height: usize) {
        for y in 0..height {
            let want = line_value(y % 2 == 0, y / 2);
            let row = &frame[y * BYTES_PER_LINE..(y + 1) * BYTES_PER_LINE];
            assert!(row.iter().all(|&b| b == want), "line {y} is wrong");
        }
    }

    #[test]
    fn weaves_fields_for_every_standard_and_packet_layout() {
        for standard in [Standard::Ntsc, Standard::Pal] {
            for vbi in [false, true] {
                let mut asm = FrameAssembler::new(standard);
                let mut frames = 0;
                let mut emit = |frame: &[u8]| {
                    assert_eq!(frame.len(), BYTES_PER_LINE * standard.height());
                    check_frame(frame, standard.height());
                    frames += 1;
                };
                feed_field(&mut asm, standard, false, 3072, vbi, &mut emit);
                for _ in 0..4 {
                    feed_field(&mut asm, standard, true, 3072, vbi, &mut emit);
                    feed_field(&mut asm, standard, false, 1448, vbi, &mut emit);
                }
                feed_field(&mut asm, standard, true, 940, vbi, &mut emit);
                assert_eq!(frames, 4, "{standard:?} vbi={vbi}");
            }
        }
    }

    #[test]
    fn ignores_data_before_the_first_header() {
        let mut asm = FrameAssembler::new(Standard::Ntsc);
        let mut frames = 0;
        asm.push(&[0xaa; 3072], &mut |_| frames += 1);
        assert_eq!(frames, 0);
        assert!(asm.frame.chunks(4).all(|c| c == BLACK_YUYV));
    }

    #[test]
    fn oversized_fields_are_clamped() {
        let mut asm = FrameAssembler::new(Standard::Ntsc);
        let mut packet = vec![0x22, 0x5a, 0, 0];
        packet.extend(std::iter::repeat_n(0x55, BYTES_PER_LINE * 300));
        asm.push(&packet, &mut |_| {});
        assert_eq!(asm.pos, BYTES_PER_LINE * 240);
    }

    fn header(kind: [u8; 2], top: bool) -> Vec<u8> {
        vec![kind[0], kind[1], u8::from(!top), 0]
    }

    fn packet(head: &[u8], fill: u8, len: usize) -> Vec<u8> {
        let mut buf = head.to_vec();
        buf.extend(std::iter::repeat_n(fill, len));
        buf
    }

    fn is_black(bytes: &[u8]) -> bool {
        bytes.chunks(4).all(|c| c == BLACK_YUYV)
    }

    const VIDEO: [u8; 2] = [0x22, 0x5a];
    const VBI: [u8; 2] = [0x33, 0x95];
    const MORE: [u8; 4] = [0x88; 4];

    #[test]
    fn the_dimensions_follow_the_standard() {
        let ntsc = FrameAssembler::new(Standard::Ntsc);
        assert_eq!((ntsc.width(), ntsc.height()), (720, 480));
        assert_eq!(ntsc.frame.len(), BYTES_PER_LINE * 480);
        let pal = FrameAssembler::new(Standard::Pal);
        assert_eq!((pal.width(), pal.height()), (720, 576));
        assert_eq!(pal.frame.len(), BYTES_PER_LINE * 576);
        assert!(is_black(&pal.frame));
    }

    #[test]
    fn an_empty_packet_changes_nothing() {
        let mut asm = FrameAssembler::new(Standard::Ntsc);
        asm.push(&header(VIDEO, true), &mut |_| {});
        asm.push(&[], &mut |_| panic!("nothing to emit"));
        assert_eq!(asm.pos, 0);
        assert_eq!(asm.state, State::Video);
    }

    #[test]
    fn a_continuation_before_any_header_is_dropped() {
        let mut asm = FrameAssembler::new(Standard::Ntsc);
        asm.push(&packet(&MORE, 0x55, 64), &mut |_| panic!("nothing to emit"));
        assert_eq!(asm.state, State::Idle);
        assert!(is_black(&asm.frame));
    }

    #[test]
    fn a_packet_shorter_than_a_header_is_video_payload() {
        let mut asm = FrameAssembler::new(Standard::Ntsc);
        asm.push(&header(VIDEO, true), &mut |_| {});
        asm.push(&[0x61, 0x62, 0x63], &mut |_| {});
        assert_eq!(asm.pos, 3);
        assert_eq!(&asm.frame[..3], &[0x61, 0x62, 0x63]);
    }

    #[test]
    fn a_header_is_only_recognised_at_the_start_of_a_packet() {
        let mut asm = FrameAssembler::new(Standard::Ntsc);
        let mut first = header(VIDEO, true);
        first.extend_from_slice(&[0x01, 0x22, 0x5a, 0x01, 0x00]);
        asm.push(&first, &mut |_| panic!("nothing to emit"));
        assert_eq!(asm.pos, 5);
        assert!(asm.writing_top);
        assert_eq!(&asm.frame[..5], &[0x01, 0x22, 0x5a, 0x01, 0x00]);
    }

    #[test]
    fn only_the_low_bit_of_the_field_byte_picks_the_field() {
        let mut asm = FrameAssembler::new(Standard::Ntsc);
        asm.push(&[0x22, 0x5a, 0xfe, 0xff], &mut |_| {});
        assert!(asm.writing_top);
        asm.push(&[0x22, 0x5a, 0x03, 0x00], &mut |_| {});
        assert!(!asm.writing_top);
    }

    #[test]
    fn a_bottom_field_fills_the_odd_lines_only() {
        let mut asm = FrameAssembler::new(Standard::Ntsc);
        asm.push(&packet(&header(VIDEO, false), 0x77, BYTES_PER_LINE * 2), &mut |_| {});
        let line = |y: usize| &asm.frame[y * BYTES_PER_LINE..(y + 1) * BYTES_PER_LINE];
        assert!(is_black(line(0)));
        assert!(line(1).iter().all(|&b| b == 0x77));
        assert!(is_black(line(2)));
        assert!(line(3).iter().all(|&b| b == 0x77));
        assert!(is_black(line(4)));
    }

    #[test]
    fn a_payload_that_straddles_a_line_wraps_to_the_next_line_of_the_field() {
        let mut asm = FrameAssembler::new(Standard::Ntsc);
        asm.push(&packet(&header(VIDEO, true), 0x31, BYTES_PER_LINE - 2), &mut |_| {});
        asm.push(&packet(&MORE, 0x32, 6), &mut |_| {});
        assert_eq!(&asm.frame[BYTES_PER_LINE - 2..BYTES_PER_LINE], &[0x32, 0x32]);
        assert!(is_black(&asm.frame[BYTES_PER_LINE..2 * BYTES_PER_LINE]));
        assert_eq!(&asm.frame[2 * BYTES_PER_LINE..2 * BYTES_PER_LINE + 4], &[0x32; 4]);
        assert!(is_black(&asm.frame[2 * BYTES_PER_LINE + 4..3 * BYTES_PER_LINE]));
    }

    #[test]
    fn the_first_top_field_is_held_until_the_next_one_begins() {
        let mut asm = FrameAssembler::new(Standard::Ntsc);
        let pushed = |asm: &mut FrameAssembler, packet: &[u8]| {
            let mut frames = Vec::new();
            asm.push(packet, &mut |f| frames.push(f.to_vec()));
            frames
        };
        assert_eq!(
            pushed(&mut asm, &packet(&header(VIDEO, true), 0x41, 16)),
            [] as [Vec<u8>; 0]
        );
        assert_eq!(
            pushed(&mut asm, &packet(&header(VIDEO, false), 0x42, 16)),
            [] as [Vec<u8>; 0]
        );
        let frames = pushed(&mut asm, &packet(&header(VIDEO, true), 0x43, 16));
        assert_eq!(frames.len(), 1);
        assert_eq!(&frames[0][..16], &[0x41; 16]);
        assert_eq!(&frames[0][BYTES_PER_LINE..BYTES_PER_LINE + 16], &[0x42; 16]);
    }

    #[test]
    fn bottom_fields_alone_never_emit_a_frame() {
        let mut asm = FrameAssembler::new(Standard::Pal);
        for _ in 0..4 {
            asm.push(&packet(&header(VIDEO, false), 0x42, 16), &mut |_| {
                panic!("nothing to emit")
            });
        }
        assert!(!asm.have_frame);
    }

    #[test]
    fn a_new_header_mid_field_restarts_the_field_from_the_first_line() {
        let mut asm = FrameAssembler::new(Standard::Ntsc);
        asm.push(&packet(&header(VIDEO, true), 0x51, BYTES_PER_LINE * 3), &mut |_| {});
        assert_eq!(asm.pos, BYTES_PER_LINE * 3);
        asm.push(&packet(&header(VIDEO, true), 0x52, 8), &mut |_| {});
        assert_eq!(asm.pos, 8);
        assert_eq!(&asm.frame[..8], &[0x52; 8]);
    }

    #[test]
    fn data_past_a_full_field_is_dropped_until_the_next_header() {
        let mut asm = FrameAssembler::new(Standard::Ntsc);
        let field = BYTES_PER_LINE * 240;
        asm.push(&packet(&header(VIDEO, true), 0x61, field), &mut |_| {});
        let before = asm.frame.clone();
        asm.push(&packet(&MORE, 0x62, 4096), &mut |_| {});
        assert_eq!(asm.pos, field);
        assert_eq!(asm.frame, before);
    }

    #[test]
    fn vbi_spread_over_packets_is_skipped_before_the_video() {
        let standard = Standard::Ntsc;
        let vbi = FRAME_WIDTH * standard.vbi_lines();
        let mut asm = FrameAssembler::new(standard);
        asm.push(&packet(&header(VBI, true), 0x11, vbi - 100), &mut |_| {});
        assert_eq!(asm.state, State::Vbi);
        assert_eq!(asm.vbi_read, vbi - 100);
        let mut rest = packet(&MORE, 0x11, 100);
        rest.extend([0x71; 10]);
        asm.push(&rest, &mut |_| {});
        assert_eq!(asm.state, State::Video);
        assert_eq!(asm.pos, 10);
        assert_eq!(&asm.frame[..10], &[0x71; 10]);
    }

    #[test]
    fn vbi_that_ends_exactly_on_a_packet_boundary_starts_video_on_the_next_packet() {
        let standard = Standard::Pal;
        let vbi = FRAME_WIDTH * standard.vbi_lines();
        let mut asm = FrameAssembler::new(standard);
        asm.push(&packet(&header(VBI, true), 0x11, vbi), &mut |_| {});
        assert_eq!(asm.state, State::Vbi);
        asm.push(&packet(&MORE, 0x72, 12), &mut |_| {});
        assert_eq!(asm.state, State::Video);
        assert_eq!(&asm.frame[..12], &[0x72; 12]);
    }

    #[test]
    fn a_header_only_vbi_packet_waits_for_more_vbi() {
        let mut asm = FrameAssembler::new(Standard::Ntsc);
        asm.push(&header(VBI, true), &mut |_| {});
        assert_eq!(asm.state, State::Vbi);
        assert_eq!(asm.vbi_read, 0);
    }

    #[test]
    fn a_vbi_header_resets_the_vbi_count() {
        let mut asm = FrameAssembler::new(Standard::Ntsc);
        asm.push(&packet(&header(VBI, true), 0x11, 500), &mut |_| {});
        asm.push(&packet(&header(VBI, false), 0x11, 200), &mut |_| {});
        assert_eq!(asm.vbi_read, 200);
        assert!(!asm.top_field);
    }

    #[test]
    fn a_field_that_arrives_short_leaves_the_rest_of_the_previous_frame() {
        let mut asm = FrameAssembler::new(Standard::Ntsc);
        let mut frames = Vec::new();
        asm.push(&packet(&header(VIDEO, true), 0x81, BYTES_PER_LINE * 240), &mut |_| {});
        asm.push(&packet(&header(VIDEO, true), 0x82, BYTES_PER_LINE), &mut |_| {});
        asm.push(&header(VIDEO, true), &mut |f| frames.push(f.to_vec()));
        assert_eq!(frames.len(), 1);
        assert!(frames[0][..BYTES_PER_LINE].iter().all(|&b| b == 0x82));
        assert!(
            frames[0][2 * BYTES_PER_LINE..3 * BYTES_PER_LINE]
                .iter()
                .all(|&b| b == 0x81)
        );
    }
}
