use std::ffi::{CStr, CString, c_char, c_void};
use std::ptr;

use crate::obs_sys as sys;

pub(crate) trait Source: Send + Sync + Sized {
    const ID: &'static CStr;
    const NAME: &'static CStr;

    fn create(settings: &Data, output: Output) -> Self;
    fn update(&self, settings: &Data);
    fn defaults(settings: &Data);
    fn properties(properties: &Properties);
    fn list_changed(settings: &Data);
}

pub(crate) fn register<S: Source>() {
    let info = sys::obs_source_info {
        id: S::ID.as_ptr(),
        type_: sys::obs_source_type::OBS_SOURCE_TYPE_INPUT,
        output_flags: sys::OBS_SOURCE_ASYNC_VIDEO | sys::OBS_SOURCE_DO_NOT_DUPLICATE,
        get_name: Some(name::<S>),
        create: Some(create::<S>),
        destroy: Some(destroy::<S>),
        update: Some(update::<S>),
        get_defaults: Some(defaults::<S>),
        get_properties: Some(properties::<S>),
        icon_type: sys::obs_icon_type::OBS_ICON_TYPE_CAMERA,
        ..Default::default()
    };
    unsafe { sys::obs_register_source_s(&raw const info, size_of::<sys::obs_source_info>()) };
}

extern "C" fn name<S: Source>(_type_data: *mut c_void) -> *const c_char {
    S::NAME.as_ptr()
}

extern "C" fn create<S: Source>(settings: *mut sys::obs_data_t, source: *mut sys::obs_source_t) -> *mut c_void {
    Box::into_raw(Box::new(S::create(&Data(settings), Output(source)))).cast()
}

extern "C" fn destroy<S: Source>(data: *mut c_void) {
    drop(unsafe { Box::from_raw(data.cast::<S>()) });
}

extern "C" fn update<S: Source>(data: *mut c_void, settings: *mut sys::obs_data_t) {
    let source = unsafe { &*data.cast::<S>() };
    source.update(&Data(settings));
}

extern "C" fn defaults<S: Source>(settings: *mut sys::obs_data_t) {
    S::defaults(&Data(settings));
}

extern "C" fn properties<S: Source>(_data: *mut c_void) -> *mut sys::obs_properties_t {
    let properties = Properties {
        raw: unsafe { sys::obs_properties_create() },
        on_list_change: Some(list_changed::<S>),
    };
    S::properties(&properties);
    properties.raw
}

extern "C" fn list_changed<S: Source>(
    _properties: *mut sys::obs_properties_t,
    _property: *mut sys::obs_property_t,
    settings: *mut sys::obs_data_t,
) -> bool {
    S::list_changed(&Data(settings));
    true
}

fn cstring(text: &str) -> CString {
    CString::new(text).expect("OBS strings have no NUL bytes")
}

pub(crate) trait Settings {
    fn int(&self, key: &str) -> i64;
    fn set_default_int(&self, key: &str, value: i64);
}

pub(crate) struct Data(*mut sys::obs_data_t);

impl Settings for Data {
    fn int(&self, key: &str) -> i64 {
        unsafe { sys::obs_data_get_int(self.0, cstring(key).as_ptr()) }
    }

    fn set_default_int(&self, key: &str, value: i64) {
        unsafe { sys::obs_data_set_default_int(self.0, cstring(key).as_ptr(), value) };
    }
}

pub(crate) struct Properties {
    raw: *mut sys::obs_properties_t,
    on_list_change: sys::obs_property_modified_t,
}

impl Properties {
    pub(crate) fn add_list(&self, name: &str, label: &str, items: &[(&str, i64)]) {
        unsafe {
            let list = sys::obs_properties_add_list(
                self.raw,
                cstring(name).as_ptr(),
                cstring(label).as_ptr(),
                sys::obs_combo_type::OBS_COMBO_TYPE_LIST,
                sys::obs_combo_format::OBS_COMBO_FORMAT_INT,
            );
            sys::obs_property_set_modified_callback(list, self.on_list_change);
            for &(item, value) in items {
                sys::obs_property_list_add_int(list, cstring(item).as_ptr(), value);
            }
        }
    }

    pub(crate) fn add_int_slider(&self, name: &str, label: &str, min: i32, max: i32) {
        unsafe {
            sys::obs_properties_add_int_slider(self.raw, cstring(name).as_ptr(), cstring(label).as_ptr(), min, max, 1);
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ColorParameters {
    matrix: [f32; 16],
    range_min: [f32; 3],
    range_max: [f32; 3],
}

impl ColorParameters {
    pub(crate) fn bt601_limited() -> Self {
        let mut p = Self {
            matrix: [0.0; 16],
            range_min: [0.0; 3],
            range_max: [0.0; 3],
        };
        unsafe {
            sys::video_format_get_parameters_for_format(
                sys::video_colorspace::VIDEO_CS_601,
                sys::video_range_type::VIDEO_RANGE_PARTIAL,
                sys::video_format::VIDEO_FORMAT_YUY2,
                p.matrix.as_mut_ptr(),
                p.range_min.as_mut_ptr(),
                p.range_max.as_mut_ptr(),
            );
        }
        p
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Output(*mut sys::obs_source_t);

unsafe impl Send for Output {}

impl Output {
    pub(crate) fn yuy2(self, data: &[u8], width: usize, height: usize, color: &ColorParameters) {
        if data.len() < width * height * 2 {
            return;
        }
        let mut frame = sys::obs_source_frame {
            width: width as u32,
            height: height as u32,
            format: sys::video_format::VIDEO_FORMAT_YUY2,
            timestamp: unsafe { sys::os_gettime_ns() },
            color_matrix: color.matrix,
            color_range_min: color.range_min,
            color_range_max: color.range_max,
            ..Default::default()
        };
        frame.data[0] = data.as_ptr().cast_mut();
        frame.linesize[0] = (width * 2) as u32;
        self.video(&raw const frame);
    }

    pub(crate) fn clear(self) {
        self.video(ptr::null());
    }

    fn video(self, frame: *const sys::obs_source_frame) {
        unsafe { sys::obs_source_output_video(self.0, frame) };
    }
}

pub(crate) mod log {
    use std::ffi::{CString, c_int};

    use crate::obs_sys as sys;

    fn write(level: sys::_bindgen_ty_1::Type, message: &str) {
        let Ok(text) = CString::new(format!("[svid2usb] {message}")) else {
            return;
        };
        unsafe { sys::blog(level as c_int, c"%s".as_ptr(), text.as_ptr()) };
    }

    pub(crate) fn info(message: &str) {
        write(sys::_bindgen_ty_1::LOG_INFO, message);
    }

    pub(crate) fn warn(message: &str) {
        write(sys::_bindgen_ty_1::LOG_WARNING, message);
    }
}

#[cfg(test)]
pub(crate) mod fake;

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::fake::{self, FakeData, Kind};
    use super::*;

    thread_local! {
        static EVENTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn event(text: String) {
        EVENTS.with(|events| events.borrow_mut().push(text));
    }

    fn events() -> Vec<String> {
        EVENTS.with(|events| std::mem::take(&mut *events.borrow_mut()))
    }

    struct Probe {
        source: usize,
        width: i64,
    }

    impl Drop for Probe {
        fn drop(&mut self) {
            event(format!("destroy {}", self.width));
        }
    }

    impl Source for Probe {
        const ID: &'static CStr = c"probe_source";
        const NAME: &'static CStr = c"Probe Source";

        fn create(settings: &Data, output: Output) -> Self {
            let width = settings.int("width");
            event(format!("create {width}"));
            Probe {
                source: output.0.addr(),
                width,
            }
        }

        fn update(&self, settings: &Data) {
            event(format!("update {} to {}", self.width, settings.int("width")));
        }

        fn defaults(settings: &Data) {
            settings.set_default_int("width", 720);
            event("defaults".to_owned());
        }

        fn properties(properties: &Properties) {
            properties.add_list("mode", "Mode", &[("First", 0), ("Second", 1)]);
            properties.add_int_slider("width", "Width", 16, 1920);
        }

        fn list_changed(settings: &Data) {
            event(format!("list changed to {}", settings.int("mode")));
        }
    }

    fn bits<const N: usize>(values: [f32; N]) -> [u32; N] {
        values.map(f32::to_bits)
    }

    fn registered() -> sys::obs_source_info {
        register::<Probe>();
        let calls = fake::take();
        assert_eq!(calls.registered.len(), 1);
        calls.registered[0].0
    }

    #[test]
    fn a_source_registers_as_an_async_video_input_with_its_size() {
        register::<Probe>();
        let calls = fake::take();
        let [(info, size)] = calls.registered[..] else {
            panic!("expected a single registration");
        };
        assert_eq!(size, size_of::<sys::obs_source_info>());
        assert_eq!(unsafe { CStr::from_ptr(info.id) }, c"probe_source");
        assert_eq!(info.type_, sys::obs_source_type::OBS_SOURCE_TYPE_INPUT);
        assert_eq!(
            info.output_flags,
            sys::OBS_SOURCE_ASYNC_VIDEO | sys::OBS_SOURCE_DO_NOT_DUPLICATE
        );
        assert_eq!(info.icon_type, sys::obs_icon_type::OBS_ICON_TYPE_CAMERA);
        let name = unsafe { info.get_name.unwrap()(ptr::null_mut()) };
        assert_eq!(unsafe { CStr::from_ptr(name) }, c"Probe Source");
    }

    #[test]
    fn only_the_implemented_callbacks_are_registered() {
        let info = registered();
        assert!(info.create.is_some());
        assert!(info.destroy.is_some());
        assert!(info.update.is_some());
        assert!(info.get_defaults.is_some());
        assert!(info.get_properties.is_some());
        assert!(info.get_width.is_none());
        assert!(info.get_height.is_none());
        assert!(info.activate.is_none());
        assert!(info.video_tick.is_none());
    }

    #[test]
    fn the_callbacks_carry_one_instance_from_create_to_destroy() {
        let info = registered();
        let created = FakeData::with(&[("width", 640)]);
        let source = ptr::without_provenance_mut(0x5eed);
        let instance = unsafe { info.create.unwrap()(created.raw(), source) };
        assert_eq!(unsafe { &*instance.cast::<Probe>() }.source, 0x5eed);

        let updated = FakeData::with(&[("width", 800)]);
        unsafe { info.update.unwrap()(instance, updated.raw()) };
        unsafe { info.destroy.unwrap()(instance) };

        assert_eq!(events(), ["create 640", "update 640 to 800", "destroy 640"]);
    }

    #[test]
    fn the_defaults_callback_writes_defaults_that_user_values_override() {
        let info = registered();
        let unset = FakeData::default();
        unsafe { info.get_defaults.unwrap()(unset.raw()) };
        assert_eq!(events(), ["defaults"]);
        assert_eq!(unset.default_of("width"), Some(720));
        assert_eq!(unset.data().int("width"), 720);

        let set = FakeData::with(&[("width", 1024)]);
        unsafe { info.get_defaults.unwrap()(set.raw()) };
        assert_eq!(set.data().int("width"), 1024);
    }

    #[test]
    fn the_properties_callback_builds_an_int_list_and_a_slider() {
        let info = registered();
        let raw = unsafe { info.get_properties.unwrap()(ptr::null_mut()) };
        let properties = fake::properties_from(raw);
        let list = properties.list.borrow();
        let [mode, width] = &list[..] else {
            panic!("expected two properties, got {:?}", properties.names());
        };

        assert_eq!((mode.name.as_str(), mode.label.as_str()), ("mode", "Mode"));
        assert_eq!(
            mode.kind,
            Kind::List {
                combo: sys::obs_combo_type::OBS_COMBO_TYPE_LIST,
                format: sys::obs_combo_format::OBS_COMBO_FORMAT_INT,
            }
        );
        assert_eq!(
            *mode.items.borrow(),
            [("First".to_owned(), 0), ("Second".to_owned(), 1)]
        );

        assert_eq!((width.name.as_str(), width.label.as_str()), ("width", "Width"));
        assert_eq!(
            width.kind,
            Kind::Slider {
                min: 16,
                max: 1920,
                step: 1
            }
        );
        assert!(width.items.borrow().is_empty());
        assert!(width.modified.borrow().is_none());
    }

    #[test]
    fn changing_a_list_calls_back_into_the_source_and_asks_for_a_refresh() {
        let info = registered();
        let raw = unsafe { info.get_properties.unwrap()(ptr::null_mut()) };
        let properties = fake::properties_from(raw);
        let list = properties.list.borrow();
        let modified = list[0].modified.borrow().expect("lists get a modified callback");

        let settings = FakeData::with(&[("mode", 1)]);
        let refresh = unsafe { modified(raw, ptr::null_mut(), settings.raw()) };

        assert!(refresh);
        assert_eq!(events(), ["list changed to 1"]);
    }

    #[test]
    #[should_panic(expected = "OBS strings have no NUL bytes")]
    fn a_setting_key_with_a_nul_byte_is_refused() {
        FakeData::default().data().int("bad\0key");
    }

    #[test]
    fn bt601_limited_asks_libobs_for_the_yuy2_parameters_and_keeps_them() {
        let color = ColorParameters::bt601_limited();
        assert_eq!(
            fake::take().colors,
            [(
                sys::video_colorspace::VIDEO_CS_601,
                sys::video_range_type::VIDEO_RANGE_PARTIAL,
                sys::video_format::VIDEO_FORMAT_YUY2,
            )]
        );
        assert_eq!(
            bits(color.matrix),
            std::array::from_fn(|i| f32::from(i as u8).to_bits())
        );
        assert_eq!(bits(color.range_min), bits([0.0625; 3]));
        assert_eq!(bits(color.range_max), bits([0.9375; 3]));
    }

    #[test]
    fn a_full_buffer_is_sent_as_one_packed_yuy2_frame() {
        let color = ColorParameters::bt601_limited();
        let pixels = vec![0x80_u8; 6 * 4 * 2];
        fake::output(0xcafe).yuy2(&pixels, 6, 4, &color);

        let calls = fake::take();
        let [(source, Some(frame))] = calls.frames[..] else {
            panic!("expected a single frame");
        };
        assert_eq!(source, 0xcafe);
        assert_eq!((frame.width, frame.height), (6, 4));
        assert_eq!(frame.format, sys::video_format::VIDEO_FORMAT_YUY2);
        assert_eq!(frame.linesize[0], 12);
        assert_eq!(frame.data[0].cast_const(), pixels.as_ptr());
        assert!(frame.data[1..].iter().all(|plane| plane.is_null()));
        assert!(frame.linesize[1..].iter().all(|&size| size == 0));
        assert_eq!(frame.timestamp, fake::NOW);
        assert_eq!(bits(frame.color_matrix), bits(color.matrix));
        assert_eq!(bits(frame.color_range_min), bits(color.range_min));
        assert_eq!(bits(frame.color_range_max), bits(color.range_max));
        assert!(!frame.full_range);
        assert!(!frame.flip);
    }

    #[test]
    fn a_buffer_longer_than_the_frame_is_still_sent() {
        let color = ColorParameters::bt601_limited();
        let pixels = vec![0_u8; 4 * 2 * 2 + 7];
        fake::output(1).yuy2(&pixels, 4, 2, &color);
        let frames = fake::take().frames;
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].1.map(|f| (f.width, f.height)), Some((4, 2)));
    }

    #[test]
    fn a_buffer_shorter_than_the_frame_is_dropped() {
        let color = ColorParameters::bt601_limited();
        fake::output(1).yuy2(&[0_u8; 4 * 2 * 2 - 1], 4, 2, &color);
        fake::output(1).yuy2(&[], 720, 480, &color);
        assert!(fake::take().frames.is_empty());
    }

    #[test]
    fn clearing_sends_a_null_frame_to_the_source() {
        fake::output(0xbeef).clear();
        let frames = fake::take().frames;
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].0, 0xbeef);
        assert!(frames[0].1.is_none());
    }

    #[test]
    fn log_levels_map_to_libobs_and_the_message_goes_through_a_format() {
        log::info("hello");
        log::warn("careful");
        assert_eq!(
            fake::take().logs,
            [
                (sys::_bindgen_ty_1::LOG_INFO as i32, "%s".to_owned()),
                (sys::_bindgen_ty_1::LOG_WARNING as i32, "%s".to_owned()),
            ]
        );
    }

    #[test]
    fn a_log_message_with_a_nul_byte_is_dropped() {
        log::warn("bad\0message");
        assert!(fake::take().logs.is_empty());
    }
}
