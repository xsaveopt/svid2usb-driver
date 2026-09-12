#[allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_qualifications,
    clippy::all,
    clippy::pedantic
)]
mod obs_sys;

mod capture;
mod obs;

use std::ffi::c_void;

use crate::obs_sys as sys;

const VERSION: &str = match option_env!("SVID2USB_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

#[unsafe(no_mangle)]
pub extern "C" fn obs_module_set_pointer(_module: *mut c_void) {}

#[unsafe(no_mangle)]
pub extern "C" fn obs_module_ver() -> u32 {
    (sys::LIBOBS_API_MAJOR_VER << 24) | (sys::LIBOBS_API_MINOR_VER << 16) | sys::LIBOBS_API_PATCH_VER
}

#[unsafe(no_mangle)]
pub extern "C" fn obs_module_load() -> bool {
    obs::register::<capture::Capture>();
    obs::log::info(&format!("loaded version {VERSION}"));
    true
}
