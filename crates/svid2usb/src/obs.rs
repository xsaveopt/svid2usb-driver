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

pub(crate) struct Data(*mut sys::obs_data_t);

impl Data {
    pub(crate) fn int(&self, key: &str) -> i64 {
        unsafe { sys::obs_data_get_int(self.0, cstring(key).as_ptr()) }
    }

    pub(crate) fn set_default_int(&self, key: &str, value: i64) {
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
