use std::ffi::CStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use em28281::{CONTROLS, Device, Input, Standard};

use crate::obs::{self, ColorParameters, Data, Output, Properties, log};

const RETRY: Duration = Duration::from_secs(1);
const POLL: Duration = Duration::from_millis(100);

type Picture = [i32; CONTROLS.len()];

#[derive(Clone, Copy, PartialEq, Eq)]
struct Config {
    input: Input,
    standard: Standard,
    picture: Picture,
}

struct Shared {
    running: AtomicBool,
    wanted: Mutex<Config>,
}

pub(crate) struct Capture {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.shared.running.store(false, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn read_selection(settings: &Data) -> (Input, Standard) {
    let input = if settings.int("input") == 1 {
        Input::SVideo
    } else {
        Input::Composite
    };
    let standard = if settings.int("standard") == 1 {
        Standard::Pal
    } else {
        Standard::Ntsc
    };
    (input, standard)
}

fn set_picture_defaults(settings: &Data, input: Input, standard: Standard) {
    for control in CONTROLS {
        settings.set_default_int(control.name, i64::from(control.default_value(input, standard)));
    }
}

fn read_config(settings: &Data) -> Config {
    let (input, standard) = read_selection(settings);
    set_picture_defaults(settings, input, standard);
    let picture = std::array::from_fn(|i| {
        let control = &CONTROLS[i];
        settings
            .int(control.name)
            .clamp(i64::from(control.min), i64::from(control.max)) as i32
    });
    Config {
        input,
        standard,
        picture,
    }
}

impl obs::Source for Capture {
    const ID: &'static CStr = c"svid2usb_capture";
    const NAME: &'static CStr = c"SVID2USB232 Capture";

    fn create(settings: &Data, output: Output) -> Self {
        let shared = Arc::new(Shared {
            running: AtomicBool::new(true),
            wanted: Mutex::new(read_config(settings)),
        });
        let worker = Arc::clone(&shared);
        let thread = std::thread::Builder::new()
            .name("svid2usb".into())
            .spawn(move || run(&worker, output))
            .map_err(|e| log::warn(&format!("could not start the capture thread: {e}")))
            .ok();
        Capture { shared, thread }
    }

    fn update(&self, settings: &Data) {
        *self.shared.wanted.lock().unwrap_or_else(PoisonError::into_inner) = read_config(settings);
    }

    fn defaults(settings: &Data) {
        settings.set_default_int("input", 0);
        settings.set_default_int("standard", 0);
        set_picture_defaults(settings, Input::default(), Standard::default());
    }

    fn properties(properties: &Properties) {
        properties.add_list("input", "Input", &[("Composite", 0), ("S-Video", 1)]);
        properties.add_list("standard", "Video standard", &[("NTSC (480i)", 0), ("PAL (576i)", 1)]);
        for control in CONTROLS {
            properties.add_int_slider(control.name, control.label, control.min, control.max);
        }
    }

    fn list_changed(settings: &Data) {
        let (input, standard) = read_selection(settings);
        set_picture_defaults(settings, input, standard);
    }
}

fn sleep_while_running(shared: &Shared, total: Duration) {
    let deadline = Instant::now() + total;
    while shared.running.load(Ordering::Acquire) && Instant::now() < deadline {
        std::thread::sleep(POLL);
    }
}

fn connect() -> em28281::Result<Device> {
    let device = Device::open()?;
    device.init()?;
    Ok(device)
}

fn apply_picture(device: &Device, picture: &Picture, previous: Option<&Picture>) -> em28281::Result<()> {
    for (i, control) in CONTROLS.iter().enumerate() {
        if previous.is_none_or(|p| p[i] != picture[i]) {
            device.set(control, picture[i])?;
        }
    }
    Ok(())
}

fn start(device: &mut Device, output: Output, config: &Config) -> em28281::Result<()> {
    device.configure(config.input, config.standard)?;
    apply_picture(device, &config.picture, None)?;
    let color = ColorParameters::bt601_limited();
    device.start(Box::new(move |data, width, height| {
        output.yuy2(data, width, height, &color);
    }))
}

fn run(shared: &Shared, output: Output) {
    let mut device: Option<Device> = None;
    let mut applied: Option<Config> = None;
    let mut last_error = String::new();

    while shared.running.load(Ordering::Acquire) {
        let Some(dev) = device.as_mut() else {
            match connect() {
                Ok(d) => {
                    log::info("device connected");
                    last_error.clear();
                    applied = None;
                    device = Some(d);
                }
                Err(e) => {
                    let message = e.to_string();
                    if message != last_error {
                        log::info(&format!("waiting for the device: {message}"));
                        last_error = message;
                    }
                    sleep_while_running(shared, RETRY);
                }
            }
            continue;
        };

        let wanted = *shared.wanted.lock().unwrap_or_else(PoisonError::into_inner);
        match applied {
            Some(current) if current == wanted => {}
            Some(current) if (current.input, current.standard) == (wanted.input, wanted.standard) => {
                if let Err(e) = apply_picture(dev, &wanted.picture, Some(&current.picture)) {
                    log::warn(&format!("applying picture controls failed: {e}"));
                }
                applied = Some(wanted);
            }
            _ => {
                if let Err(e) = start(dev, output, &wanted) {
                    log::warn(&format!("starting capture failed: {e}"));
                    device = None;
                    sleep_while_running(shared, RETRY);
                    continue;
                }
                applied = Some(wanted);
            }
        }

        if let Err(e) = dev.poll(POLL) {
            log::warn(&format!("capture stopped: {e}"));
            device = None;
            output.clear();
        }
    }
}
