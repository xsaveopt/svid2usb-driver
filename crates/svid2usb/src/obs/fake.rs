use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, c_char, c_int, c_longlong};
use std::ptr;
use std::rc::Rc;

use super::{Data, Output};
use crate::obs_sys as sys;

#[derive(Default)]
pub(crate) struct Calls {
    pub(crate) registered: Vec<(sys::obs_source_info, usize)>,
    pub(crate) logs: Vec<(c_int, String)>,
    pub(crate) frames: Vec<(usize, Option<sys::obs_source_frame>)>,
    pub(crate) colors: Vec<(u32, u32, u32)>,
}

thread_local! {
    static CALLS: RefCell<Calls> = RefCell::default();
}

pub(crate) fn take() -> Calls {
    CALLS.with(|calls| std::mem::take(&mut *calls.borrow_mut()))
}

fn record(f: impl FnOnce(&mut Calls)) {
    CALLS.with(|calls| f(&mut calls.borrow_mut()));
}

fn text(raw: *const c_char) -> String {
    unsafe { CStr::from_ptr(raw) }.to_str().unwrap().to_owned()
}

#[derive(Default)]
pub(crate) struct FakeData {
    values: HashMap<String, i64>,
    defaults: RefCell<HashMap<String, i64>>,
}

impl FakeData {
    pub(crate) fn with(values: &[(&str, i64)]) -> Self {
        FakeData {
            values: values.iter().map(|&(k, v)| (k.to_owned(), v)).collect(),
            defaults: RefCell::default(),
        }
    }

    pub(crate) fn raw(&self) -> *mut sys::obs_data_t {
        ptr::from_ref(self).cast_mut().cast()
    }

    pub(crate) fn data(&self) -> Data {
        Data(self.raw())
    }

    pub(crate) fn default_of(&self, key: &str) -> Option<i64> {
        self.defaults.borrow().get(key).copied()
    }
}

pub(crate) fn output(address: usize) -> Output {
    Output(ptr::without_provenance_mut(address))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    List { combo: u32, format: u32 },
    Slider { min: i32, max: i32, step: i32 },
}

pub(crate) struct FakeProperty {
    pub(crate) name: String,
    pub(crate) label: String,
    pub(crate) kind: Kind,
    pub(crate) items: RefCell<Vec<(String, i64)>>,
    pub(crate) modified: RefCell<sys::obs_property_modified_t>,
}

#[derive(Default)]
pub(crate) struct FakeProperties {
    pub(crate) list: RefCell<Vec<Rc<FakeProperty>>>,
}

impl FakeProperties {
    pub(crate) fn names(&self) -> Vec<String> {
        self.list.borrow().iter().map(|p| p.name.clone()).collect()
    }
}

pub(crate) fn properties_from(raw: *mut sys::obs_properties_t) -> FakeProperties {
    *unsafe { Box::from_raw(raw.cast::<FakeProperties>()) }
}

fn add(
    props: *mut sys::obs_properties_t,
    name: *const c_char,
    label: *const c_char,
    kind: Kind,
) -> *mut sys::obs_property_t {
    let props = unsafe { &*props.cast::<FakeProperties>() };
    let property = Rc::new(FakeProperty {
        name: text(name),
        label: text(label),
        kind,
        items: RefCell::default(),
        modified: RefCell::new(None),
    });
    let raw = Rc::as_ptr(&property).cast_mut().cast();
    props.list.borrow_mut().push(property);
    raw
}

fn property<'a>(raw: *mut sys::obs_property_t) -> &'a FakeProperty {
    unsafe { &*raw.cast::<FakeProperty>() }
}

#[unsafe(no_mangle)]
extern "C" fn obs_register_source_s(info: *const sys::obs_source_info, size: usize) {
    let info = unsafe { *info };
    record(|calls| calls.registered.push((info, size)));
}

#[unsafe(no_mangle)]
extern "C" fn obs_data_get_int(data: *mut sys::obs_data_t, name: *const c_char) -> c_longlong {
    let data = unsafe { &*data.cast::<FakeData>() };
    let key = text(name);
    data.values
        .get(&key)
        .copied()
        .or_else(|| data.default_of(&key))
        .unwrap_or(0)
}

#[unsafe(no_mangle)]
extern "C" fn obs_data_set_default_int(data: *mut sys::obs_data_t, name: *const c_char, value: c_longlong) {
    let data = unsafe { &*data.cast::<FakeData>() };
    data.defaults.borrow_mut().insert(text(name), value);
}

#[unsafe(no_mangle)]
extern "C" fn obs_properties_create() -> *mut sys::obs_properties_t {
    Box::into_raw(Box::<FakeProperties>::default()).cast()
}

#[unsafe(no_mangle)]
extern "C" fn obs_properties_add_list(
    props: *mut sys::obs_properties_t,
    name: *const c_char,
    label: *const c_char,
    combo: sys::obs_combo_type::Type,
    format: sys::obs_combo_format::Type,
) -> *mut sys::obs_property_t {
    add(props, name, label, Kind::List { combo, format })
}

#[unsafe(no_mangle)]
extern "C" fn obs_properties_add_int_slider(
    props: *mut sys::obs_properties_t,
    name: *const c_char,
    label: *const c_char,
    min: c_int,
    max: c_int,
    step: c_int,
) -> *mut sys::obs_property_t {
    add(props, name, label, Kind::Slider { min, max, step })
}

#[unsafe(no_mangle)]
extern "C" fn obs_property_set_modified_callback(p: *mut sys::obs_property_t, modified: sys::obs_property_modified_t) {
    *property(p).modified.borrow_mut() = modified;
}

#[unsafe(no_mangle)]
extern "C" fn obs_property_list_add_int(p: *mut sys::obs_property_t, name: *const c_char, value: c_longlong) -> usize {
    let mut items = property(p).items.borrow_mut();
    items.push((text(name), value));
    items.len() - 1
}

#[unsafe(no_mangle)]
extern "C" fn video_format_get_parameters_for_format(
    color_space: sys::video_colorspace::Type,
    range: sys::video_range_type::Type,
    format: sys::video_format::Type,
    matrix: *mut f32,
    min_range: *mut f32,
    max_range: *mut f32,
) -> bool {
    record(|calls| calls.colors.push((color_space, range, format)));
    unsafe {
        for i in 0..16_u8 {
            *matrix.add(usize::from(i)) = f32::from(i);
        }
        for i in 0..3 {
            *min_range.add(i) = 0.0625;
            *max_range.add(i) = 0.9375;
        }
    }
    true
}

pub(crate) const NOW: u64 = 1_234_567_890;

#[unsafe(no_mangle)]
extern "C" fn os_gettime_ns() -> u64 {
    NOW
}

#[unsafe(no_mangle)]
extern "C" fn obs_source_output_video(source: *mut sys::obs_source_t, frame: *const sys::obs_source_frame) {
    let frame = unsafe { frame.as_ref() }.copied();
    record(|calls| calls.frames.push((source.addr(), frame)));
}

#[unsafe(no_mangle)]
extern "C" fn blog(level: c_int, format: *const c_char) {
    let format = text(format);
    record(|calls| calls.logs.push((level, format)));
}
