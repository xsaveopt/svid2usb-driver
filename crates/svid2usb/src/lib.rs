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

#[cfg(test)]
mod tests {
    use std::ffi::CStr;
    use std::ptr;

    use super::*;
    use crate::obs::fake;

    #[test]
    fn the_module_version_packs_the_libobs_api_it_was_built_against() {
        let version = obs_module_ver();
        assert_eq!(version >> 24, sys::LIBOBS_API_MAJOR_VER);
        assert_eq!((version >> 16) & 0xff, sys::LIBOBS_API_MINOR_VER);
        assert_eq!(version & 0xffff, sys::LIBOBS_API_PATCH_VER);
    }

    #[test]
    fn the_module_pointer_is_accepted_even_when_null() {
        obs_module_set_pointer(ptr::null_mut());
        let calls = fake::take();
        assert!(calls.registered.is_empty());
        assert!(calls.logs.is_empty());
    }

    #[test]
    fn loading_registers_the_capture_source_once_and_logs_it() {
        assert!(obs_module_load());
        let calls = fake::take();
        let [(info, _)] = calls.registered[..] else {
            panic!("expected a single registration");
        };
        assert_eq!(unsafe { CStr::from_ptr(info.id) }, c"svid2usb_capture");
        let name = unsafe { info.get_name.unwrap()(ptr::null_mut()) };
        assert_eq!(unsafe { CStr::from_ptr(name) }, c"SVID2USB232 Capture");
        assert_eq!(calls.logs, [(sys::_bindgen_ty_1::LOG_INFO as i32, "%s".to_owned())]);
    }
}
