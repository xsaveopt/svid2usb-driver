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
}
