use std::ffi::CStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use em28281::{CONTROLS, Device, Input, Standard};

use crate::obs::{self, ColorParameters, Data, Output, Properties, Settings, log};

const RETRY: Duration = Duration::from_secs(1);
const POLL: Duration = Duration::from_millis(100);

type Picture = [i32; CONTROLS.len()];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

fn read_selection(settings: &impl Settings) -> (Input, Standard) {
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

fn set_picture_defaults(settings: &impl Settings, input: Input, standard: Standard) {
    for control in CONTROLS {
        settings.set_default_int(control.name, i64::from(control.default_value(input, standard)));
    }
}

fn read_config(settings: &impl Settings) -> Config {
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
            .spawn(move || run(&worker, &mut Usb { output, device: None }, &SystemClock))
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

trait Clock {
    fn now(&self) -> Instant;
    fn sleep(&self, duration: Duration);
}

struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

trait Backend {
    fn connect(&mut self) -> em28281::Result<()>;
    fn start(&mut self, config: &Config) -> em28281::Result<()>;
    fn apply_picture(&mut self, picture: &Picture, previous: &Picture) -> em28281::Result<()>;
    fn poll(&mut self) -> em28281::Result<()>;
    fn disconnect(&mut self);
    fn clear(&mut self);
    fn log_info(&mut self, message: &str);
    fn log_warn(&mut self, message: &str);
}

struct Usb {
    output: Output,
    device: Option<Device>,
}

impl Usb {
    fn device(&mut self) -> em28281::Result<&mut Device> {
        self.device.as_mut().ok_or(em28281::Error::NotFound)
    }
}

impl Backend for Usb {
    fn connect(&mut self) -> em28281::Result<()> {
        let device = Device::open()?;
        device.init()?;
        self.device = Some(device);
        Ok(())
    }

    fn start(&mut self, config: &Config) -> em28281::Result<()> {
        let output = self.output;
        let color = ColorParameters::bt601_limited();
        let device = self.device()?;
        device.configure(config.input, config.standard)?;
        apply_picture(device, &config.picture, None)?;
        device.start(Box::new(move |data, width, height| {
            output.yuy2(data, width, height, &color);
        }))
    }

    fn apply_picture(&mut self, picture: &Picture, previous: &Picture) -> em28281::Result<()> {
        apply_picture(self.device()?, picture, Some(previous))
    }

    fn poll(&mut self) -> em28281::Result<()> {
        self.device()?.poll(POLL)
    }

    fn disconnect(&mut self) {
        self.device = None;
    }

    fn clear(&mut self) {
        self.output.clear();
    }

    fn log_info(&mut self, message: &str) {
        log::info(message);
    }

    fn log_warn(&mut self, message: &str) {
        log::warn(message);
    }
}

fn sleep_while_running(shared: &Shared, total: Duration, clock: &impl Clock) {
    let deadline = clock.now() + total;
    while shared.running.load(Ordering::Acquire) && clock.now() < deadline {
        clock.sleep(POLL);
    }
}

fn apply_picture(device: &Device, picture: &Picture, previous: Option<&Picture>) -> em28281::Result<()> {
    for (i, control) in CONTROLS.iter().enumerate() {
        if previous.is_none_or(|p| p[i] != picture[i]) {
            device.set(control, picture[i])?;
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Idle,
    Picture(Picture),
    Restart,
}

fn action(applied: Option<&Config>, wanted: &Config) -> Action {
    match applied {
        Some(current) if current == wanted => Action::Idle,
        Some(current) if (current.input, current.standard) == (wanted.input, wanted.standard) => {
            Action::Picture(current.picture)
        }
        _ => Action::Restart,
    }
}

fn run(shared: &Shared, backend: &mut impl Backend, clock: &impl Clock) {
    let mut connected = false;
    let mut applied: Option<Config> = None;
    let mut last_error = String::new();

    while shared.running.load(Ordering::Acquire) {
        if !connected {
            match backend.connect() {
                Ok(()) => {
                    backend.log_info("device connected");
                    last_error.clear();
                    applied = None;
                    connected = true;
                }
                Err(e) => {
                    let message = e.to_string();
                    if message != last_error {
                        backend.log_info(&format!("waiting for the device: {message}"));
                        last_error = message;
                    }
                    sleep_while_running(shared, RETRY, clock);
                }
            }
            continue;
        }

        let wanted = *shared.wanted.lock().unwrap_or_else(PoisonError::into_inner);
        match action(applied.as_ref(), &wanted) {
            Action::Idle => {}
            Action::Picture(previous) => {
                if let Err(e) = backend.apply_picture(&wanted.picture, &previous) {
                    backend.log_warn(&format!("applying picture controls failed: {e}"));
                }
                applied = Some(wanted);
            }
            Action::Restart => {
                if let Err(e) = backend.start(&wanted) {
                    backend.log_warn(&format!("starting capture failed: {e}"));
                    backend.disconnect();
                    connected = false;
                    sleep_while_running(shared, RETRY, clock);
                    continue;
                }
                applied = Some(wanted);
            }
        }

        if let Err(e) = backend.poll() {
            backend.log_warn(&format!("capture stopped: {e}"));
            backend.disconnect();
            connected = false;
            backend.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::collections::{HashMap, VecDeque};

    use em28281::Error;

    use super::*;
    use crate::obs::fake::{self, FakeData, Kind};

    struct FakeSettings {
        values: HashMap<String, i64>,
        defaults: RefCell<HashMap<String, i64>>,
    }

    impl FakeSettings {
        fn new(values: &[(&str, i64)]) -> Self {
            FakeSettings {
                values: values.iter().map(|&(k, v)| (k.to_owned(), v)).collect(),
                defaults: RefCell::new(HashMap::new()),
            }
        }

        fn default_of(&self, key: &str) -> Option<i64> {
            self.defaults.borrow().get(key).copied()
        }
    }

    impl Settings for FakeSettings {
        fn int(&self, key: &str) -> i64 {
            self.values
                .get(key)
                .copied()
                .or_else(|| self.default_of(key))
                .unwrap_or(0)
        }

        fn set_default_int(&self, key: &str, value: i64) {
            self.defaults.borrow_mut().insert(key.to_owned(), value);
        }
    }

    struct FakeClock {
        now: Cell<Instant>,
        slept: RefCell<Vec<Duration>>,
        shared: Option<Arc<Shared>>,
        stop_at: usize,
    }

    impl FakeClock {
        fn new() -> Self {
            FakeClock {
                now: Cell::new(Instant::now()),
                slept: RefCell::new(Vec::new()),
                shared: None,
                stop_at: usize::MAX,
            }
        }

        fn stopping(shared: &Arc<Shared>, after: usize) -> Self {
            FakeClock {
                shared: Some(Arc::clone(shared)),
                stop_at: after,
                ..FakeClock::new()
            }
        }

        fn sleeps(&self) -> Vec<Duration> {
            self.slept.borrow().clone()
        }

        fn elapsed(&self, since: Instant) -> Duration {
            self.now.get() - since
        }
    }

    impl Clock for FakeClock {
        fn now(&self) -> Instant {
            self.now.get()
        }

        fn sleep(&self, duration: Duration) {
            self.now.set(self.now.get() + duration);
            let ticks = {
                let mut slept = self.slept.borrow_mut();
                slept.push(duration);
                slept.len()
            };
            if ticks >= self.stop_at
                && let Some(shared) = &self.shared
            {
                shared.running.store(false, Ordering::Release);
            }
        }
    }

    #[derive(Default)]
    struct Script {
        connect: Vec<em28281::Result<()>>,
        start: Vec<em28281::Result<()>>,
        picture: Vec<em28281::Result<()>>,
        poll: Vec<em28281::Result<()>>,
        updates: Vec<Config>,
    }

    struct Fake {
        shared: Arc<Shared>,
        remaining: usize,
        connect: VecDeque<em28281::Result<()>>,
        start: VecDeque<em28281::Result<()>>,
        picture: VecDeque<em28281::Result<()>>,
        poll: VecDeque<em28281::Result<()>>,
        updates: VecDeque<Config>,
        events: Vec<String>,
        started: Vec<Config>,
        pictures: Vec<(Picture, Picture)>,
    }

    impl Fake {
        fn new(shared: &Arc<Shared>, script: Script) -> Self {
            Fake {
                shared: Arc::clone(shared),
                remaining: script.connect.len() + script.start.len() + script.picture.len() + script.poll.len(),
                connect: script.connect.into(),
                start: script.start.into(),
                picture: script.picture.into(),
                poll: script.poll.into(),
                updates: script.updates.into(),
                events: Vec::new(),
                started: Vec::new(),
                pictures: Vec::new(),
            }
        }

        fn tick(&mut self, event: &str) {
            self.events.push(event.to_owned());
            self.remaining -= 1;
            if self.remaining == 0 {
                self.shared.running.store(false, Ordering::Release);
            }
        }
    }

    impl Backend for Fake {
        fn connect(&mut self) -> em28281::Result<()> {
            self.tick("connect");
            self.connect.pop_front().expect("the script covers every connect")
        }

        fn start(&mut self, config: &Config) -> em28281::Result<()> {
            self.tick("start");
            self.started.push(*config);
            self.start.pop_front().expect("the script covers every start")
        }

        fn apply_picture(&mut self, picture: &Picture, previous: &Picture) -> em28281::Result<()> {
            self.tick("picture");
            self.pictures.push((*picture, *previous));
            self.picture
                .pop_front()
                .expect("the script covers every picture change")
        }

        fn poll(&mut self) -> em28281::Result<()> {
            self.tick("poll");
            if let Some(update) = self.updates.pop_front() {
                *self.shared.wanted.lock().unwrap_or_else(PoisonError::into_inner) = update;
            }
            self.poll.pop_front().expect("the script covers every poll")
        }

        fn disconnect(&mut self) {
            self.events.push("disconnect".to_owned());
        }

        fn clear(&mut self) {
            self.events.push("clear".to_owned());
        }

        fn log_info(&mut self, message: &str) {
            self.events.push(format!("info: {message}"));
        }

        fn log_warn(&mut self, message: &str) {
            self.events.push(format!("warn: {message}"));
        }
    }

    fn config(input: Input, standard: Standard) -> Config {
        Config {
            input,
            standard,
            picture: [0; CONTROLS.len()],
        }
    }

    fn shared_with(wanted: Config) -> Arc<Shared> {
        Arc::new(Shared {
            running: AtomicBool::new(true),
            wanted: Mutex::new(wanted),
        })
    }

    fn lines(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    fn ticks(total: Duration) -> usize {
        (total.as_nanos() / POLL.as_nanos()) as usize
    }

    #[test]
    fn a_selection_that_is_not_the_second_option_is_the_default_one() {
        let settings = FakeSettings::new(&[]);
        assert_eq!(read_selection(&settings), (Input::Composite, Standard::Ntsc));

        let settings = FakeSettings::new(&[("input", 1), ("standard", 1)]);
        assert_eq!(read_selection(&settings), (Input::SVideo, Standard::Pal));

        let settings = FakeSettings::new(&[("input", 7), ("standard", -3)]);
        assert_eq!(read_selection(&settings), (Input::Composite, Standard::Ntsc));
    }

    #[test]
    fn every_control_is_given_a_default_for_the_selection() {
        let settings = FakeSettings::new(&[]);
        set_picture_defaults(&settings, Input::SVideo, Standard::Pal);
        for control in CONTROLS {
            assert_eq!(
                settings.default_of(control.name),
                Some(i64::from(control.default_value(Input::SVideo, Standard::Pal))),
                "{} has no default",
                control.name
            );
        }
    }

    #[test]
    fn defaults_are_rewritten_when_the_selection_changes() {
        let settings = FakeSettings::new(&[]);
        set_picture_defaults(&settings, Input::Composite, Standard::Ntsc);
        let ntsc = settings.default_of("brightness");
        set_picture_defaults(&settings, Input::Composite, Standard::Pal);
        assert_ne!(ntsc, settings.default_of("brightness"));
    }

    #[test]
    fn an_unset_picture_reads_back_as_the_defaults_for_the_selection() {
        let settings = FakeSettings::new(&[("input", 1), ("standard", 1)]);
        let config = read_config(&settings);
        assert_eq!(config.input, Input::SVideo);
        assert_eq!(config.standard, Standard::Pal);
        let expected: Picture = std::array::from_fn(|i| CONTROLS[i].default_value(Input::SVideo, Standard::Pal));
        assert_eq!(config.picture, expected);
    }

    #[test]
    fn a_picture_value_outside_the_control_range_is_clamped() {
        let settings = FakeSettings::new(&[("hue", 5000), ("brightness", -5000), ("sharpness", i64::MIN)]);
        let config = read_config(&settings);
        let value = |name: &str| {
            let i = CONTROLS.iter().position(|c| c.name == name).unwrap();
            config.picture[i]
        };
        assert_eq!(value("hue"), 127);
        assert_eq!(value("brightness"), 0);
        assert_eq!(value("sharpness"), 0);
    }

    #[test]
    fn an_unchanged_config_asks_for_nothing() {
        let wanted = config(Input::Composite, Standard::Ntsc);
        assert_eq!(action(Some(&wanted), &wanted), Action::Idle);
    }

    #[test]
    fn a_picture_only_difference_asks_for_the_picture_alone() {
        let applied = config(Input::SVideo, Standard::Pal);
        let mut wanted = applied;
        wanted.picture[0] = 42;
        assert_eq!(action(Some(&applied), &wanted), Action::Picture(applied.picture));
    }

    #[test]
    fn a_new_input_or_standard_asks_for_a_restart() {
        let applied = config(Input::Composite, Standard::Ntsc);
        assert_eq!(
            action(Some(&applied), &config(Input::SVideo, Standard::Ntsc)),
            Action::Restart
        );
        assert_eq!(
            action(Some(&applied), &config(Input::Composite, Standard::Pal)),
            Action::Restart
        );
        assert_eq!(action(None, &applied), Action::Restart);
    }

    #[test]
    fn a_wait_runs_out_the_whole_retry_delay() {
        let shared = shared_with(config(Input::Composite, Standard::Ntsc));
        let clock = FakeClock::new();
        let start = clock.now();
        sleep_while_running(&shared, RETRY, &clock);
        assert_eq!(clock.sleeps(), vec![POLL; ticks(RETRY)]);
        assert_eq!(clock.elapsed(start), RETRY);
    }

    #[test]
    fn a_wait_ends_as_soon_as_the_capture_is_stopped() {
        let shared = shared_with(config(Input::Composite, Standard::Ntsc));
        let clock = FakeClock::stopping(&shared, 3);
        sleep_while_running(&shared, RETRY, &clock);
        assert_eq!(clock.sleeps().len(), 3);
    }

    #[test]
    fn a_stopped_capture_never_starts_a_wait() {
        let shared = shared_with(config(Input::Composite, Standard::Ntsc));
        shared.running.store(false, Ordering::Release);
        let clock = FakeClock::new();
        sleep_while_running(&shared, RETRY, &clock);
        assert!(clock.sleeps().is_empty());
    }

    #[test]
    fn a_missing_device_is_retried_and_logged_once_per_distinct_reason() {
        let shared = shared_with(config(Input::Composite, Standard::Ntsc));
        let mut fake = Fake::new(
            &shared,
            Script {
                connect: vec![Err(Error::NotFound), Err(Error::NotFound), Err(Error::Disconnected)],
                ..Script::default()
            },
        );
        let clock = FakeClock::new();
        run(&shared, &mut fake, &clock);

        let missing = format!("info: waiting for the device: {}", Error::NotFound);
        let gone = format!("info: waiting for the device: {}", Error::Disconnected);
        assert_eq!(fake.events, lines(&["connect", &missing, "connect", "connect", &gone]));
        assert_eq!(clock.sleeps().len(), 2 * ticks(RETRY));
    }

    #[test]
    fn a_connected_device_is_started_once_and_then_polled() {
        let shared = shared_with(config(Input::Composite, Standard::Ntsc));
        let mut fake = Fake::new(
            &shared,
            Script {
                connect: vec![Ok(())],
                start: vec![Ok(())],
                poll: vec![Ok(()), Ok(())],
                ..Script::default()
            },
        );
        let clock = FakeClock::new();
        run(&shared, &mut fake, &clock);

        assert_eq!(
            fake.events,
            lines(&["connect", "info: device connected", "start", "poll", "poll"])
        );
        assert_eq!(fake.started, vec![config(Input::Composite, Standard::Ntsc)]);
        assert!(clock.sleeps().is_empty());
    }

    #[test]
    fn a_poll_failure_drops_the_device_and_clears_the_output() {
        let shared = shared_with(config(Input::Composite, Standard::Ntsc));
        let mut fake = Fake::new(
            &shared,
            Script {
                connect: vec![Ok(())],
                start: vec![Ok(())],
                poll: vec![Err(Error::Disconnected)],
                ..Script::default()
            },
        );
        let clock = FakeClock::new();
        run(&shared, &mut fake, &clock);

        let stopped = format!("warn: capture stopped: {}", Error::Disconnected);
        assert_eq!(
            fake.events,
            lines(&[
                "connect",
                "info: device connected",
                "start",
                "poll",
                &stopped,
                "disconnect",
                "clear",
            ])
        );
        assert!(clock.sleeps().is_empty());
    }

    #[test]
    fn a_failed_start_drops_the_connection_and_waits_before_reconnecting() {
        let shared = shared_with(config(Input::Composite, Standard::Ntsc));
        let mut fake = Fake::new(
            &shared,
            Script {
                connect: vec![Ok(()), Ok(())],
                start: vec![Err(Error::NoVideoInterface), Ok(())],
                poll: vec![Ok(())],
                ..Script::default()
            },
        );
        let clock = FakeClock::new();
        run(&shared, &mut fake, &clock);

        let failed = format!("warn: starting capture failed: {}", Error::NoVideoInterface);
        assert_eq!(
            fake.events,
            lines(&[
                "connect",
                "info: device connected",
                "start",
                &failed,
                "disconnect",
                "connect",
                "info: device connected",
                "start",
                "poll",
            ])
        );
        assert_eq!(clock.sleeps().len(), ticks(RETRY));
    }

    #[test]
    fn a_picture_change_is_applied_against_the_previous_picture_without_restarting() {
        let base = config(Input::Composite, Standard::Ntsc);
        let mut changed = base;
        changed.picture[3] = 17;
        let shared = shared_with(base);
        let mut fake = Fake::new(
            &shared,
            Script {
                connect: vec![Ok(())],
                start: vec![Ok(())],
                picture: vec![Ok(())],
                poll: vec![Ok(()), Ok(())],
                updates: vec![changed],
            },
        );
        run(&shared, &mut fake, &FakeClock::new());

        assert_eq!(
            fake.events,
            lines(&["connect", "info: device connected", "start", "poll", "picture", "poll"])
        );
        assert_eq!(fake.started, vec![base]);
        assert_eq!(fake.pictures, vec![(changed.picture, base.picture)]);
    }

    #[test]
    fn a_failed_picture_change_is_only_logged() {
        let base = config(Input::Composite, Standard::Ntsc);
        let mut changed = base;
        changed.picture[0] = 9;
        let shared = shared_with(base);
        let mut fake = Fake::new(
            &shared,
            Script {
                connect: vec![Ok(())],
                start: vec![Ok(())],
                picture: vec![Err(Error::ShortTransfer(0x7a09))],
                poll: vec![Ok(()), Ok(())],
                updates: vec![changed],
            },
        );
        run(&shared, &mut fake, &FakeClock::new());

        let failed = format!(
            "warn: applying picture controls failed: {}",
            Error::ShortTransfer(0x7a09)
        );
        assert_eq!(
            fake.events,
            lines(&[
                "connect",
                "info: device connected",
                "start",
                "poll",
                "picture",
                &failed,
                "poll",
            ])
        );
        assert_eq!(fake.started, vec![base]);
    }

    #[test]
    fn a_new_standard_restarts_the_capture_on_the_same_connection() {
        let base = config(Input::Composite, Standard::Ntsc);
        let pal = config(Input::Composite, Standard::Pal);
        let shared = shared_with(base);
        let mut fake = Fake::new(
            &shared,
            Script {
                connect: vec![Ok(())],
                start: vec![Ok(()), Ok(())],
                poll: vec![Ok(()), Ok(())],
                updates: vec![pal],
                ..Script::default()
            },
        );
        run(&shared, &mut fake, &FakeClock::new());

        assert_eq!(
            fake.events,
            lines(&["connect", "info: device connected", "start", "poll", "start", "poll"])
        );
        assert_eq!(fake.started, vec![base, pal]);
    }

    fn registered() -> crate::obs_sys::obs_source_info {
        obs::register::<Capture>();
        fake::take().registered[0].0
    }

    #[test]
    fn the_settings_ui_offers_input_standard_and_every_picture_control() {
        let info = registered();
        let raw = unsafe { info.get_properties.unwrap()(std::ptr::null_mut()) };
        let properties = fake::properties_from(raw);
        let list = properties.list.borrow();

        let mut expected = vec!["input".to_owned(), "standard".to_owned()];
        expected.extend(CONTROLS.iter().map(|c| c.name.to_owned()));
        assert_eq!(properties.names(), expected);

        assert_eq!(list[0].label, "Input");
        assert_eq!(
            *list[0].items.borrow(),
            [("Composite".to_owned(), 0), ("S-Video".to_owned(), 1)]
        );
        assert_eq!(list[1].label, "Video standard");
        assert_eq!(
            *list[1].items.borrow(),
            [("NTSC (480i)".to_owned(), 0), ("PAL (576i)".to_owned(), 1)]
        );
        assert!(list[..2].iter().all(|p| p.modified.borrow().is_some()));

        for (property, control) in list[2..].iter().zip(CONTROLS) {
            assert_eq!(property.label, control.label);
            assert_eq!(
                property.kind,
                Kind::Slider {
                    min: control.min,
                    max: control.max,
                    step: 1
                }
            );
        }
    }

    #[test]
    fn the_list_values_match_what_the_settings_are_read_as() {
        let info = registered();
        let properties = fake::properties_from(unsafe { info.get_properties.unwrap()(std::ptr::null_mut()) });
        let list = properties.list.borrow();
        let second = |i: usize| list[i].items.borrow()[1].1;
        let settings = FakeData::with(&[("input", second(0)), ("standard", second(1))]);
        assert_eq!(read_selection(&settings.data()), (Input::SVideo, Standard::Pal));
    }

    #[test]
    fn the_defaults_callback_starts_on_composite_ntsc() {
        let info = registered();
        let settings = FakeData::default();
        unsafe { info.get_defaults.unwrap()(settings.raw()) };
        assert_eq!(settings.default_of("input"), Some(0));
        assert_eq!(settings.default_of("standard"), Some(0));
        for control in CONTROLS {
            assert_eq!(
                settings.default_of(control.name),
                Some(i64::from(control.default_value(Input::Composite, Standard::Ntsc))),
                "{}",
                control.name
            );
        }
    }

    #[test]
    fn switching_the_selection_in_the_ui_moves_the_picture_defaults_along() {
        let info = registered();
        let raw = unsafe { info.get_properties.unwrap()(std::ptr::null_mut()) };
        let properties = fake::properties_from(raw);
        let modified = properties.list.borrow()[1].modified.borrow().unwrap();

        let settings = FakeData::with(&[("input", 1), ("standard", 1)]);
        assert!(unsafe { modified(raw, std::ptr::null_mut(), settings.raw()) });
        for control in CONTROLS {
            assert_eq!(
                settings.default_of(control.name),
                Some(i64::from(control.default_value(Input::SVideo, Standard::Pal))),
                "{}",
                control.name
            );
        }
    }

    #[test]
    fn an_update_replaces_the_wanted_config() {
        let capture = Capture {
            shared: shared_with(config(Input::Composite, Standard::Ntsc)),
            thread: None,
        };
        let settings = FakeData::with(&[("input", 1), ("standard", 1), ("hue", 5000)]);
        obs::Source::update(&capture, &settings.data());

        let wanted = *capture.shared.wanted.lock().unwrap();
        assert_eq!((wanted.input, wanted.standard), (Input::SVideo, Standard::Pal));
        assert_eq!(wanted.picture, read_config(&settings.data()).picture);
        assert_eq!(wanted.picture[3], 127);
    }

    #[test]
    fn dropping_a_capture_stops_its_worker_and_waits_for_it() {
        let shared = shared_with(config(Input::Composite, Standard::Ntsc));
        let finished = Arc::new(AtomicBool::new(false));
        let worker = Arc::clone(&shared);
        let done = Arc::clone(&finished);
        let thread = std::thread::spawn(move || {
            while worker.running.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(1));
            }
            done.store(true, Ordering::Release);
        });
        drop(Capture {
            shared: Arc::clone(&shared),
            thread: Some(thread),
        });
        assert!(!shared.running.load(Ordering::Acquire));
        assert!(finished.load(Ordering::Acquire));
    }

    #[test]
    fn the_usb_backend_without_a_device_reports_it_as_missing() {
        let mut usb = Usb {
            output: fake::output(0x1),
            device: None,
        };
        let picture = [0; CONTROLS.len()];
        assert!(matches!(
            usb.start(&config(Input::Composite, Standard::Ntsc)),
            Err(Error::NotFound)
        ));
        assert!(matches!(usb.apply_picture(&picture, &picture), Err(Error::NotFound)));
        assert!(matches!(usb.poll(), Err(Error::NotFound)));
        usb.disconnect();
        assert!(usb.device.is_none());
        assert!(fake::take().frames.is_empty());
    }

    #[test]
    fn the_usb_backend_clears_and_logs_through_obs() {
        let mut usb = Usb {
            output: fake::output(0xabc),
            device: None,
        };
        usb.clear();
        usb.log_info("up");
        usb.log_warn("down");
        let calls = fake::take();
        assert_eq!(calls.frames.len(), 1);
        assert_eq!(calls.frames[0].0, 0xabc);
        assert!(calls.frames[0].1.is_none());
        let levels: Vec<_> = calls.logs.iter().map(|&(level, _)| level).collect();
        assert_eq!(
            levels,
            [
                crate::obs_sys::_bindgen_ty_1::LOG_INFO as i32,
                crate::obs_sys::_bindgen_ty_1::LOG_WARNING as i32,
            ]
        );
    }

    #[test]
    fn a_capture_that_is_stopped_before_it_begins_touches_nothing() {
        let shared = shared_with(config(Input::Composite, Standard::Ntsc));
        shared.running.store(false, Ordering::Release);
        let mut fake = Fake::new(&shared, Script::default());
        run(&shared, &mut fake, &FakeClock::new());
        assert!(fake.events.is_empty());
    }
}
