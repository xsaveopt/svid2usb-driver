use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{DeviceId, ErrorKind, SampleFormat, Stream, SupportedStreamConfig};

use crate::obs::{AudioFormat, Output, log};
use crate::obs_sys as sys;

const RETRY: Duration = Duration::from_secs(1);
const POLL: Duration = Duration::from_millis(100);

struct Shared {
    running: AtomicBool,
    lost: AtomicBool,
    wanted: Mutex<String>,
}

pub(crate) struct Audio {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for Audio {
    fn drop(&mut self) {
        self.shared.running.store(false, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Audio {
    pub(crate) fn start(output: Output, device: String) -> Self {
        let shared = Arc::new(Shared {
            running: AtomicBool::new(true),
            lost: AtomicBool::new(false),
            wanted: Mutex::new(device),
        });
        let worker = Arc::clone(&shared);
        let thread = std::thread::Builder::new()
            .name("svid2usb-audio".into())
            .spawn(move || run(&worker, output))
            .map_err(|e| log::warn(&format!("could not start the audio thread: {e}")))
            .ok();
        Audio { shared, thread }
    }

    pub(crate) fn select(&self, device: String) {
        *self.shared.wanted.lock().unwrap_or_else(PoisonError::into_inner) = device;
    }

    #[cfg(test)]
    pub(crate) fn idle() -> Self {
        Audio {
            shared: Arc::new(Shared {
                running: AtomicBool::new(false),
                lost: AtomicBool::new(false),
                wanted: Mutex::new(String::new()),
            }),
            thread: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn wanted(&self) -> String {
        self.shared.wanted.lock().unwrap().clone()
    }
}

pub(crate) fn devices() -> Vec<(String, String)> {
    let Ok(inputs) = cpal::default_host().input_devices() else {
        return Vec::new();
    };
    inputs
        .filter_map(|device| {
            let id = device.id().ok()?;
            let name = device.description().ok()?.name().to_owned();
            Some((name, id.to_string()))
        })
        .collect()
}

fn format(config: &SupportedStreamConfig) -> Option<AudioFormat> {
    let sample = match config.sample_format() {
        SampleFormat::U8 => sys::audio_format::AUDIO_FORMAT_U8BIT,
        SampleFormat::I16 => sys::audio_format::AUDIO_FORMAT_16BIT,
        SampleFormat::I32 => sys::audio_format::AUDIO_FORMAT_32BIT,
        SampleFormat::F32 => sys::audio_format::AUDIO_FORMAT_FLOAT,
        _ => return None,
    };
    let speakers = match config.channels() {
        1 => sys::speaker_layout::SPEAKERS_MONO,
        2 => sys::speaker_layout::SPEAKERS_STEREO,
        _ => return None,
    };
    Some(AudioFormat {
        sample,
        sample_size: config.sample_format().sample_size(),
        channels: usize::from(config.channels()),
        speakers,
        rate: config.sample_rate(),
    })
}

fn open(id: &str, output: Output, shared: &Arc<Shared>) -> Result<Stream, String> {
    let id: DeviceId = id.parse().map_err(|e| format!("{e}"))?;
    let device = cpal::default_host()
        .device_by_id(&id)
        .ok_or_else(|| "the audio device is not connected".to_owned())?;
    let config = device.default_input_config().map_err(|e| e.to_string())?;
    let obs = format(&config).ok_or_else(|| {
        format!(
            "unsupported audio format {} with {} channels",
            config.sample_format(),
            config.channels()
        )
    })?;
    let lost = Arc::clone(shared);
    let stream = device
        .build_input_stream_raw(
            config.config(),
            config.sample_format(),
            move |data, info| {
                let timestamp = info.timestamp();
                let latency = timestamp.callback.saturating_duration_since(timestamp.capture);
                output.audio(data.bytes(), &obs, latency);
            },
            move |e| {
                if matches!(
                    e.kind(),
                    ErrorKind::DeviceNotAvailable | ErrorKind::DeviceChanged | ErrorKind::StreamInvalidated
                ) {
                    lost.lost.store(true, Ordering::Release);
                }
            },
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}

fn sleep_while_running(shared: &Shared, total: Duration) {
    let mut waited = Duration::ZERO;
    while shared.running.load(Ordering::Acquire) && waited < total {
        std::thread::sleep(POLL);
        waited += POLL;
    }
}

fn run(shared: &Arc<Shared>, output: Output) {
    let mut current: Option<(String, Stream)> = None;
    let mut last_error = String::new();

    while shared.running.load(Ordering::Acquire) {
        let wanted = shared.wanted.lock().unwrap_or_else(PoisonError::into_inner).clone();
        let lost = shared.lost.swap(false, Ordering::AcqRel);
        if lost {
            log::warn("audio device lost");
        }
        let stale = lost || current.as_ref().is_some_and(|(id, _)| *id != wanted);
        if stale {
            current = None;
        }

        if current.is_none() && !wanted.is_empty() {
            match open(&wanted, output, shared) {
                Ok(stream) => {
                    log::info("audio device connected");
                    last_error.clear();
                    current = Some((wanted, stream));
                }
                Err(message) => {
                    if message != last_error {
                        log::info(&format!("waiting for the audio device: {message}"));
                        last_error = message;
                    }
                    sleep_while_running(shared, RETRY);
                    continue;
                }
            }
        }

        std::thread::sleep(POLL);
    }
}

#[cfg(test)]
mod tests {
    use cpal::SupportedBufferSize;

    use super::*;

    fn config(channels: u16, sample: SampleFormat) -> SupportedStreamConfig {
        SupportedStreamConfig::new(channels, 48_000, SupportedBufferSize::Unknown, sample)
    }

    #[test]
    fn interleaved_integer_and_float_samples_map_to_obs_formats() {
        let cases = [
            (SampleFormat::U8, sys::audio_format::AUDIO_FORMAT_U8BIT, 1),
            (SampleFormat::I16, sys::audio_format::AUDIO_FORMAT_16BIT, 2),
            (SampleFormat::I32, sys::audio_format::AUDIO_FORMAT_32BIT, 4),
            (SampleFormat::F32, sys::audio_format::AUDIO_FORMAT_FLOAT, 4),
        ];
        for (sample, obs, size) in cases {
            let format = format(&config(2, sample)).unwrap();
            assert_eq!(format.sample, obs, "{sample}");
            assert_eq!(format.sample_size, size, "{sample}");
            assert_eq!(format.rate, 48_000);
        }
    }

    #[test]
    fn mono_and_stereo_map_to_their_speaker_layouts() {
        let mono = format(&config(1, SampleFormat::I16)).unwrap();
        assert_eq!((mono.channels, mono.speakers), (1, sys::speaker_layout::SPEAKERS_MONO));
        let stereo = format(&config(2, SampleFormat::I16)).unwrap();
        assert_eq!(
            (stereo.channels, stereo.speakers),
            (2, sys::speaker_layout::SPEAKERS_STEREO)
        );
    }

    #[test]
    fn formats_obs_cannot_take_are_refused() {
        assert!(format(&config(2, SampleFormat::F64)).is_none());
        assert!(format(&config(2, SampleFormat::I24)).is_none());
        assert!(format(&config(6, SampleFormat::F32)).is_none());
    }

    #[test]
    fn selecting_a_device_replaces_the_wanted_one() {
        let audio = Audio::idle();
        audio.select("coreaudio:cable".to_owned());
        assert_eq!(audio.wanted(), "coreaudio:cable");
    }
}
