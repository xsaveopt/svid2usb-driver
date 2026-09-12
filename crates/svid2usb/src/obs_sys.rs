pub const LIBOBS_API_MAJOR_VER: u32 = 32;
pub const LIBOBS_API_MINOR_VER: u32 = 2;
pub const LIBOBS_API_PATCH_VER: u32 = 2;
pub const OBS_SOURCE_ASYNC_VIDEO: u32 = 5;
pub const OBS_SOURCE_DO_NOT_DUPLICATE: u32 = 128;
pub mod _bindgen_ty_1 {
    #[allow(unused_imports)]
    use super::*;
    pub type Type = ::std::os::raw::c_uint;
    pub const LOG_ERROR: Type = 100;
    pub const LOG_WARNING: Type = 200;
    pub const LOG_INFO: Type = 300;
    pub const LOG_DEBUG: Type = 400;
}
unsafe extern "C" {
    pub fn blog(log_level: ::std::os::raw::c_int, format: *const ::std::os::raw::c_char, ...);
}
pub mod gs_color_space {
    #[allow(unused_imports)]
    use super::*;
    pub type Type = ::std::os::raw::c_uint;
    pub const GS_CS_SRGB: Type = 0;
    pub const GS_CS_SRGB_16F: Type = 1;
    pub const GS_CS_709_EXTENDED: Type = 2;
    pub const GS_CS_709_SCRGB: Type = 3;
}
#[repr(C)]
#[derive(Debug)]
pub struct gs_effect {
    _unused: [u8; 0],
}
pub type gs_effect_t = gs_effect;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct audio_output_data {
    pub data: [*mut f32; 8usize],
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of audio_output_data"][::std::mem::size_of::<audio_output_data>() - 64usize];
    ["Alignment of audio_output_data"][::std::mem::align_of::<audio_output_data>() - 8usize];
    ["Offset of field: audio_output_data::data"][::std::mem::offset_of!(audio_output_data, data) - 0usize];
};
impl Default for audio_output_data {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}
pub mod video_format {
    #[allow(unused_imports)]
    use super::*;
    pub type Type = ::std::os::raw::c_uint;
    pub const VIDEO_FORMAT_NONE: Type = 0;
    pub const VIDEO_FORMAT_I420: Type = 1;
    pub const VIDEO_FORMAT_NV12: Type = 2;
    pub const VIDEO_FORMAT_YVYU: Type = 3;
    pub const VIDEO_FORMAT_YUY2: Type = 4;
    pub const VIDEO_FORMAT_UYVY: Type = 5;
    pub const VIDEO_FORMAT_RGBA: Type = 6;
    pub const VIDEO_FORMAT_BGRA: Type = 7;
    pub const VIDEO_FORMAT_BGRX: Type = 8;
    pub const VIDEO_FORMAT_Y800: Type = 9;
    pub const VIDEO_FORMAT_I444: Type = 10;
    pub const VIDEO_FORMAT_BGR3: Type = 11;
    pub const VIDEO_FORMAT_I422: Type = 12;
    pub const VIDEO_FORMAT_I40A: Type = 13;
    pub const VIDEO_FORMAT_I42A: Type = 14;
    pub const VIDEO_FORMAT_YUVA: Type = 15;
    pub const VIDEO_FORMAT_AYUV: Type = 16;
    pub const VIDEO_FORMAT_I010: Type = 17;
    pub const VIDEO_FORMAT_P010: Type = 18;
    pub const VIDEO_FORMAT_I210: Type = 19;
    pub const VIDEO_FORMAT_I412: Type = 20;
    pub const VIDEO_FORMAT_YA2L: Type = 21;
    pub const VIDEO_FORMAT_P216: Type = 22;
    pub const VIDEO_FORMAT_P416: Type = 23;
    pub const VIDEO_FORMAT_V210: Type = 24;
    pub const VIDEO_FORMAT_R10L: Type = 25;
}
pub mod video_colorspace {
    #[allow(unused_imports)]
    use super::*;
    pub type Type = ::std::os::raw::c_uint;
    pub const VIDEO_CS_DEFAULT: Type = 0;
    pub const VIDEO_CS_601: Type = 1;
    pub const VIDEO_CS_709: Type = 2;
    pub const VIDEO_CS_SRGB: Type = 3;
    pub const VIDEO_CS_2100_PQ: Type = 4;
    pub const VIDEO_CS_2100_HLG: Type = 5;
}
pub mod video_range_type {
    #[allow(unused_imports)]
    use super::*;
    pub type Type = ::std::os::raw::c_uint;
    pub const VIDEO_RANGE_DEFAULT: Type = 0;
    pub const VIDEO_RANGE_PARTIAL: Type = 1;
    pub const VIDEO_RANGE_FULL: Type = 2;
}
unsafe extern "C" {
    pub fn video_format_get_parameters_for_format(
        color_space: video_colorspace::Type,
        range: video_range_type::Type,
        format: video_format::Type,
        matrix: *mut f32,
        min_range: *mut f32,
        max_range: *mut f32,
    ) -> bool;
}
#[repr(C)]
#[derive(Debug)]
pub struct obs_data {
    _unused: [u8; 0],
}
pub type obs_data_t = obs_data;
unsafe extern "C" {
    pub fn obs_data_set_default_int(
        data: *mut obs_data_t,
        name: *const ::std::os::raw::c_char,
        val: ::std::os::raw::c_longlong,
    );
}
unsafe extern "C" {
    pub fn obs_data_get_int(data: *mut obs_data_t, name: *const ::std::os::raw::c_char) -> ::std::os::raw::c_longlong;
}
pub mod obs_combo_format {
    #[allow(unused_imports)]
    use super::*;
    pub type Type = ::std::os::raw::c_uint;
    pub const OBS_COMBO_FORMAT_INVALID: Type = 0;
    pub const OBS_COMBO_FORMAT_INT: Type = 1;
    pub const OBS_COMBO_FORMAT_FLOAT: Type = 2;
    pub const OBS_COMBO_FORMAT_STRING: Type = 3;
    pub const OBS_COMBO_FORMAT_BOOL: Type = 4;
}
pub mod obs_combo_type {
    #[allow(unused_imports)]
    use super::*;
    pub type Type = ::std::os::raw::c_uint;
    pub const OBS_COMBO_TYPE_INVALID: Type = 0;
    pub const OBS_COMBO_TYPE_EDITABLE: Type = 1;
    pub const OBS_COMBO_TYPE_LIST: Type = 2;
    pub const OBS_COMBO_TYPE_RADIO: Type = 3;
}
#[repr(C)]
#[derive(Debug)]
pub struct obs_properties {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug)]
pub struct obs_property {
    _unused: [u8; 0],
}
pub type obs_properties_t = obs_properties;
pub type obs_property_t = obs_property;
unsafe extern "C" {
    pub fn obs_properties_create() -> *mut obs_properties_t;
}
unsafe extern "C" {
    pub fn obs_properties_add_int_slider(
        props: *mut obs_properties_t,
        name: *const ::std::os::raw::c_char,
        description: *const ::std::os::raw::c_char,
        min: ::std::os::raw::c_int,
        max: ::std::os::raw::c_int,
        step: ::std::os::raw::c_int,
    ) -> *mut obs_property_t;
}
unsafe extern "C" {
    pub fn obs_properties_add_list(
        props: *mut obs_properties_t,
        name: *const ::std::os::raw::c_char,
        description: *const ::std::os::raw::c_char,
        type_: obs_combo_type::Type,
        format: obs_combo_format::Type,
    ) -> *mut obs_property_t;
}
pub type obs_property_modified_t = ::std::option::Option<
    unsafe extern "C" fn(
        props: *mut obs_properties_t,
        property: *mut obs_property_t,
        settings: *mut obs_data_t,
    ) -> bool,
>;
unsafe extern "C" {
    pub fn obs_property_set_modified_callback(p: *mut obs_property_t, modified: obs_property_modified_t);
}
unsafe extern "C" {
    pub fn obs_property_list_add_int(
        p: *mut obs_property_t,
        name: *const ::std::os::raw::c_char,
        val: ::std::os::raw::c_longlong,
    ) -> usize;
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct obs_mouse_event {
    pub modifiers: u32,
    pub x: i32,
    pub y: i32,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of obs_mouse_event"][::std::mem::size_of::<obs_mouse_event>() - 12usize];
    ["Alignment of obs_mouse_event"][::std::mem::align_of::<obs_mouse_event>() - 4usize];
    ["Offset of field: obs_mouse_event::modifiers"][::std::mem::offset_of!(obs_mouse_event, modifiers) - 0usize];
    ["Offset of field: obs_mouse_event::x"][::std::mem::offset_of!(obs_mouse_event, x) - 4usize];
    ["Offset of field: obs_mouse_event::y"][::std::mem::offset_of!(obs_mouse_event, y) - 8usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct obs_key_event {
    pub modifiers: u32,
    pub text: *mut ::std::os::raw::c_char,
    pub native_modifiers: u32,
    pub native_scancode: u32,
    pub native_vkey: u32,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of obs_key_event"][::std::mem::size_of::<obs_key_event>() - 32usize];
    ["Alignment of obs_key_event"][::std::mem::align_of::<obs_key_event>() - 8usize];
    ["Offset of field: obs_key_event::modifiers"][::std::mem::offset_of!(obs_key_event, modifiers) - 0usize];
    ["Offset of field: obs_key_event::text"][::std::mem::offset_of!(obs_key_event, text) - 8usize];
    ["Offset of field: obs_key_event::native_modifiers"]
        [::std::mem::offset_of!(obs_key_event, native_modifiers) - 16usize];
    ["Offset of field: obs_key_event::native_scancode"]
        [::std::mem::offset_of!(obs_key_event, native_scancode) - 20usize];
    ["Offset of field: obs_key_event::native_vkey"][::std::mem::offset_of!(obs_key_event, native_vkey) - 24usize];
};
impl Default for obs_key_event {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}
#[repr(C)]
#[derive(Debug)]
pub struct obs_source {
    _unused: [u8; 0],
}
pub type obs_source_t = obs_source;
#[repr(C)]
#[derive(Debug)]
pub struct obs_missing_files {
    _unused: [u8; 0],
}
pub type obs_missing_files_t = obs_missing_files;
pub mod obs_source_type {
    #[allow(unused_imports)]
    use super::*;
    pub type Type = ::std::os::raw::c_uint;
    pub const OBS_SOURCE_TYPE_INPUT: Type = 0;
    pub const OBS_SOURCE_TYPE_FILTER: Type = 1;
    pub const OBS_SOURCE_TYPE_TRANSITION: Type = 2;
    pub const OBS_SOURCE_TYPE_SCENE: Type = 3;
}
pub mod obs_icon_type {
    #[allow(unused_imports)]
    use super::*;
    pub type Type = ::std::os::raw::c_uint;
    pub const OBS_ICON_TYPE_UNKNOWN: Type = 0;
    pub const OBS_ICON_TYPE_IMAGE: Type = 1;
    pub const OBS_ICON_TYPE_COLOR: Type = 2;
    pub const OBS_ICON_TYPE_SLIDESHOW: Type = 3;
    pub const OBS_ICON_TYPE_AUDIO_INPUT: Type = 4;
    pub const OBS_ICON_TYPE_AUDIO_OUTPUT: Type = 5;
    pub const OBS_ICON_TYPE_DESKTOP_CAPTURE: Type = 6;
    pub const OBS_ICON_TYPE_WINDOW_CAPTURE: Type = 7;
    pub const OBS_ICON_TYPE_GAME_CAPTURE: Type = 8;
    pub const OBS_ICON_TYPE_CAMERA: Type = 9;
    pub const OBS_ICON_TYPE_TEXT: Type = 10;
    pub const OBS_ICON_TYPE_MEDIA: Type = 11;
    pub const OBS_ICON_TYPE_BROWSER: Type = 12;
    pub const OBS_ICON_TYPE_CUSTOM: Type = 13;
    pub const OBS_ICON_TYPE_PROCESS_AUDIO_OUTPUT: Type = 14;
}
pub mod obs_media_state {
    #[allow(unused_imports)]
    use super::*;
    pub type Type = ::std::os::raw::c_uint;
    pub const OBS_MEDIA_STATE_NONE: Type = 0;
    pub const OBS_MEDIA_STATE_PLAYING: Type = 1;
    pub const OBS_MEDIA_STATE_OPENING: Type = 2;
    pub const OBS_MEDIA_STATE_BUFFERING: Type = 3;
    pub const OBS_MEDIA_STATE_PAUSED: Type = 4;
    pub const OBS_MEDIA_STATE_STOPPED: Type = 5;
    pub const OBS_MEDIA_STATE_ENDED: Type = 6;
    pub const OBS_MEDIA_STATE_ERROR: Type = 7;
}
pub type obs_source_enum_proc_t = ::std::option::Option<
    unsafe extern "C" fn(parent: *mut obs_source_t, child: *mut obs_source_t, param: *mut ::std::os::raw::c_void),
>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct obs_source_audio_mix {
    pub output: [audio_output_data; 6usize],
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of obs_source_audio_mix"][::std::mem::size_of::<obs_source_audio_mix>() - 384usize];
    ["Alignment of obs_source_audio_mix"][::std::mem::align_of::<obs_source_audio_mix>() - 8usize];
    ["Offset of field: obs_source_audio_mix::output"][::std::mem::offset_of!(obs_source_audio_mix, output) - 0usize];
};
impl Default for obs_source_audio_mix {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct obs_source_info {
    pub id: *const ::std::os::raw::c_char,
    pub type_: obs_source_type::Type,
    pub output_flags: u32,
    pub get_name: ::std::option::Option<
        unsafe extern "C" fn(type_data: *mut ::std::os::raw::c_void) -> *const ::std::os::raw::c_char,
    >,
    pub create: ::std::option::Option<
        unsafe extern "C" fn(settings: *mut obs_data_t, source: *mut obs_source_t) -> *mut ::std::os::raw::c_void,
    >,
    pub destroy: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void)>,
    pub get_width: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void) -> u32>,
    pub get_height: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void) -> u32>,
    pub get_defaults: ::std::option::Option<unsafe extern "C" fn(settings: *mut obs_data_t)>,
    pub get_properties:
        ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void) -> *mut obs_properties_t>,
    pub update:
        ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void, settings: *mut obs_data_t)>,
    pub activate: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void)>,
    pub deactivate: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void)>,
    pub show: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void)>,
    pub hide: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void)>,
    pub video_tick: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void, seconds: f32)>,
    pub video_render:
        ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void, effect: *mut gs_effect_t)>,
    pub filter_video: ::std::option::Option<
        unsafe extern "C" fn(data: *mut ::std::os::raw::c_void, frame: *mut obs_source_frame) -> *mut obs_source_frame,
    >,
    pub filter_audio: ::std::option::Option<
        unsafe extern "C" fn(data: *mut ::std::os::raw::c_void, audio: *mut obs_audio_data) -> *mut obs_audio_data,
    >,
    pub enum_active_sources: ::std::option::Option<
        unsafe extern "C" fn(
            data: *mut ::std::os::raw::c_void,
            enum_callback: obs_source_enum_proc_t,
            param: *mut ::std::os::raw::c_void,
        ),
    >,
    pub save: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void, settings: *mut obs_data_t)>,
    pub load: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void, settings: *mut obs_data_t)>,
    pub mouse_click: ::std::option::Option<
        unsafe extern "C" fn(
            data: *mut ::std::os::raw::c_void,
            event: *const obs_mouse_event,
            type_: i32,
            mouse_up: bool,
            click_count: u32,
        ),
    >,
    pub mouse_move: ::std::option::Option<
        unsafe extern "C" fn(data: *mut ::std::os::raw::c_void, event: *const obs_mouse_event, mouse_leave: bool),
    >,
    pub mouse_wheel: ::std::option::Option<
        unsafe extern "C" fn(
            data: *mut ::std::os::raw::c_void,
            event: *const obs_mouse_event,
            x_delta: ::std::os::raw::c_int,
            y_delta: ::std::os::raw::c_int,
        ),
    >,
    pub focus: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void, focus: bool)>,
    pub key_click: ::std::option::Option<
        unsafe extern "C" fn(data: *mut ::std::os::raw::c_void, event: *const obs_key_event, key_up: bool),
    >,
    pub filter_remove:
        ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void, source: *mut obs_source_t)>,
    pub type_data: *mut ::std::os::raw::c_void,
    pub free_type_data: ::std::option::Option<unsafe extern "C" fn(type_data: *mut ::std::os::raw::c_void)>,
    pub audio_render: ::std::option::Option<
        unsafe extern "C" fn(
            data: *mut ::std::os::raw::c_void,
            ts_out: *mut u64,
            audio_output: *mut obs_source_audio_mix,
            mixers: u32,
            channels: usize,
            sample_rate: usize,
        ) -> bool,
    >,
    pub enum_all_sources: ::std::option::Option<
        unsafe extern "C" fn(
            data: *mut ::std::os::raw::c_void,
            enum_callback: obs_source_enum_proc_t,
            param: *mut ::std::os::raw::c_void,
        ),
    >,
    pub transition_start: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void)>,
    pub transition_stop: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void)>,
    pub get_defaults2:
        ::std::option::Option<unsafe extern "C" fn(type_data: *mut ::std::os::raw::c_void, settings: *mut obs_data_t)>,
    pub get_properties2: ::std::option::Option<
        unsafe extern "C" fn(
            data: *mut ::std::os::raw::c_void,
            type_data: *mut ::std::os::raw::c_void,
        ) -> *mut obs_properties_t,
    >,
    pub audio_mix: ::std::option::Option<
        unsafe extern "C" fn(
            data: *mut ::std::os::raw::c_void,
            ts_out: *mut u64,
            audio_output: *mut audio_output_data,
            channels: usize,
            sample_rate: usize,
        ) -> bool,
    >,
    pub icon_type: obs_icon_type::Type,
    pub media_play_pause: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void, pause: bool)>,
    pub media_restart: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void)>,
    pub media_stop: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void)>,
    pub media_next: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void)>,
    pub media_previous: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void)>,
    pub media_get_duration: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void) -> i64>,
    pub media_get_time: ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void) -> i64>,
    pub media_set_time:
        ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void, miliseconds: i64)>,
    pub media_get_state:
        ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void) -> obs_media_state::Type>,
    pub version: u32,
    pub unversioned_id: *const ::std::os::raw::c_char,
    pub missing_files:
        ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void) -> *mut obs_missing_files_t>,
    pub video_get_color_space: ::std::option::Option<
        unsafe extern "C" fn(
            data: *mut ::std::os::raw::c_void,
            count: usize,
            preferred_spaces: *const gs_color_space::Type,
        ) -> gs_color_space::Type,
    >,
    pub filter_add:
        ::std::option::Option<unsafe extern "C" fn(data: *mut ::std::os::raw::c_void, source: *mut obs_source_t)>,
    pub get_dark_icon: ::std::option::Option<
        unsafe extern "C" fn(type_data: *mut ::std::os::raw::c_void) -> *const ::std::os::raw::c_char,
    >,
    pub get_light_icon: ::std::option::Option<
        unsafe extern "C" fn(type_data: *mut ::std::os::raw::c_void) -> *const ::std::os::raw::c_char,
    >,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of obs_source_info"][::std::mem::size_of::<obs_source_info>() - 424usize];
    ["Alignment of obs_source_info"][::std::mem::align_of::<obs_source_info>() - 8usize];
    ["Offset of field: obs_source_info::id"][::std::mem::offset_of!(obs_source_info, id) - 0usize];
    ["Offset of field: obs_source_info::type_"][::std::mem::offset_of!(obs_source_info, type_) - 8usize];
    ["Offset of field: obs_source_info::output_flags"][::std::mem::offset_of!(obs_source_info, output_flags) - 12usize];
    ["Offset of field: obs_source_info::get_name"][::std::mem::offset_of!(obs_source_info, get_name) - 16usize];
    ["Offset of field: obs_source_info::create"][::std::mem::offset_of!(obs_source_info, create) - 24usize];
    ["Offset of field: obs_source_info::destroy"][::std::mem::offset_of!(obs_source_info, destroy) - 32usize];
    ["Offset of field: obs_source_info::get_width"][::std::mem::offset_of!(obs_source_info, get_width) - 40usize];
    ["Offset of field: obs_source_info::get_height"][::std::mem::offset_of!(obs_source_info, get_height) - 48usize];
    ["Offset of field: obs_source_info::get_defaults"][::std::mem::offset_of!(obs_source_info, get_defaults) - 56usize];
    ["Offset of field: obs_source_info::get_properties"]
        [::std::mem::offset_of!(obs_source_info, get_properties) - 64usize];
    ["Offset of field: obs_source_info::update"][::std::mem::offset_of!(obs_source_info, update) - 72usize];
    ["Offset of field: obs_source_info::activate"][::std::mem::offset_of!(obs_source_info, activate) - 80usize];
    ["Offset of field: obs_source_info::deactivate"][::std::mem::offset_of!(obs_source_info, deactivate) - 88usize];
    ["Offset of field: obs_source_info::show"][::std::mem::offset_of!(obs_source_info, show) - 96usize];
    ["Offset of field: obs_source_info::hide"][::std::mem::offset_of!(obs_source_info, hide) - 104usize];
    ["Offset of field: obs_source_info::video_tick"][::std::mem::offset_of!(obs_source_info, video_tick) - 112usize];
    ["Offset of field: obs_source_info::video_render"]
        [::std::mem::offset_of!(obs_source_info, video_render) - 120usize];
    ["Offset of field: obs_source_info::filter_video"]
        [::std::mem::offset_of!(obs_source_info, filter_video) - 128usize];
    ["Offset of field: obs_source_info::filter_audio"]
        [::std::mem::offset_of!(obs_source_info, filter_audio) - 136usize];
    ["Offset of field: obs_source_info::enum_active_sources"]
        [::std::mem::offset_of!(obs_source_info, enum_active_sources) - 144usize];
    ["Offset of field: obs_source_info::save"][::std::mem::offset_of!(obs_source_info, save) - 152usize];
    ["Offset of field: obs_source_info::load"][::std::mem::offset_of!(obs_source_info, load) - 160usize];
    ["Offset of field: obs_source_info::mouse_click"][::std::mem::offset_of!(obs_source_info, mouse_click) - 168usize];
    ["Offset of field: obs_source_info::mouse_move"][::std::mem::offset_of!(obs_source_info, mouse_move) - 176usize];
    ["Offset of field: obs_source_info::mouse_wheel"][::std::mem::offset_of!(obs_source_info, mouse_wheel) - 184usize];
    ["Offset of field: obs_source_info::focus"][::std::mem::offset_of!(obs_source_info, focus) - 192usize];
    ["Offset of field: obs_source_info::key_click"][::std::mem::offset_of!(obs_source_info, key_click) - 200usize];
    ["Offset of field: obs_source_info::filter_remove"]
        [::std::mem::offset_of!(obs_source_info, filter_remove) - 208usize];
    ["Offset of field: obs_source_info::type_data"][::std::mem::offset_of!(obs_source_info, type_data) - 216usize];
    ["Offset of field: obs_source_info::free_type_data"]
        [::std::mem::offset_of!(obs_source_info, free_type_data) - 224usize];
    ["Offset of field: obs_source_info::audio_render"]
        [::std::mem::offset_of!(obs_source_info, audio_render) - 232usize];
    ["Offset of field: obs_source_info::enum_all_sources"]
        [::std::mem::offset_of!(obs_source_info, enum_all_sources) - 240usize];
    ["Offset of field: obs_source_info::transition_start"]
        [::std::mem::offset_of!(obs_source_info, transition_start) - 248usize];
    ["Offset of field: obs_source_info::transition_stop"]
        [::std::mem::offset_of!(obs_source_info, transition_stop) - 256usize];
    ["Offset of field: obs_source_info::get_defaults2"]
        [::std::mem::offset_of!(obs_source_info, get_defaults2) - 264usize];
    ["Offset of field: obs_source_info::get_properties2"]
        [::std::mem::offset_of!(obs_source_info, get_properties2) - 272usize];
    ["Offset of field: obs_source_info::audio_mix"][::std::mem::offset_of!(obs_source_info, audio_mix) - 280usize];
    ["Offset of field: obs_source_info::icon_type"][::std::mem::offset_of!(obs_source_info, icon_type) - 288usize];
    ["Offset of field: obs_source_info::media_play_pause"]
        [::std::mem::offset_of!(obs_source_info, media_play_pause) - 296usize];
    ["Offset of field: obs_source_info::media_restart"]
        [::std::mem::offset_of!(obs_source_info, media_restart) - 304usize];
    ["Offset of field: obs_source_info::media_stop"][::std::mem::offset_of!(obs_source_info, media_stop) - 312usize];
    ["Offset of field: obs_source_info::media_next"][::std::mem::offset_of!(obs_source_info, media_next) - 320usize];
    ["Offset of field: obs_source_info::media_previous"]
        [::std::mem::offset_of!(obs_source_info, media_previous) - 328usize];
    ["Offset of field: obs_source_info::media_get_duration"]
        [::std::mem::offset_of!(obs_source_info, media_get_duration) - 336usize];
    ["Offset of field: obs_source_info::media_get_time"]
        [::std::mem::offset_of!(obs_source_info, media_get_time) - 344usize];
    ["Offset of field: obs_source_info::media_set_time"]
        [::std::mem::offset_of!(obs_source_info, media_set_time) - 352usize];
    ["Offset of field: obs_source_info::media_get_state"]
        [::std::mem::offset_of!(obs_source_info, media_get_state) - 360usize];
    ["Offset of field: obs_source_info::version"][::std::mem::offset_of!(obs_source_info, version) - 368usize];
    ["Offset of field: obs_source_info::unversioned_id"]
        [::std::mem::offset_of!(obs_source_info, unversioned_id) - 376usize];
    ["Offset of field: obs_source_info::missing_files"]
        [::std::mem::offset_of!(obs_source_info, missing_files) - 384usize];
    ["Offset of field: obs_source_info::video_get_color_space"]
        [::std::mem::offset_of!(obs_source_info, video_get_color_space) - 392usize];
    ["Offset of field: obs_source_info::filter_add"][::std::mem::offset_of!(obs_source_info, filter_add) - 400usize];
    ["Offset of field: obs_source_info::get_dark_icon"]
        [::std::mem::offset_of!(obs_source_info, get_dark_icon) - 408usize];
    ["Offset of field: obs_source_info::get_light_icon"]
        [::std::mem::offset_of!(obs_source_info, get_light_icon) - 416usize];
};
impl Default for obs_source_info {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}
unsafe extern "C" {
    pub fn obs_register_source_s(info: *const obs_source_info, size: usize);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct obs_audio_data {
    pub data: [*mut u8; 8usize],
    pub frames: u32,
    pub timestamp: u64,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of obs_audio_data"][::std::mem::size_of::<obs_audio_data>() - 80usize];
    ["Alignment of obs_audio_data"][::std::mem::align_of::<obs_audio_data>() - 8usize];
    ["Offset of field: obs_audio_data::data"][::std::mem::offset_of!(obs_audio_data, data) - 0usize];
    ["Offset of field: obs_audio_data::frames"][::std::mem::offset_of!(obs_audio_data, frames) - 64usize];
    ["Offset of field: obs_audio_data::timestamp"][::std::mem::offset_of!(obs_audio_data, timestamp) - 72usize];
};
impl Default for obs_audio_data {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct obs_source_frame {
    pub data: [*mut u8; 8usize],
    pub linesize: [u32; 8usize],
    pub width: u32,
    pub height: u32,
    pub timestamp: u64,
    pub format: video_format::Type,
    pub color_matrix: [f32; 16usize],
    pub full_range: bool,
    pub max_luminance: u16,
    pub color_range_min: [f32; 3usize],
    pub color_range_max: [f32; 3usize],
    pub flip: bool,
    pub flags: u8,
    pub trc: u8,
    pub refs: ::std::os::raw::c_long,
    pub prev_frame: bool,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of obs_source_frame"][::std::mem::size_of::<obs_source_frame>() - 232usize];
    ["Alignment of obs_source_frame"][::std::mem::align_of::<obs_source_frame>() - 8usize];
    ["Offset of field: obs_source_frame::data"][::std::mem::offset_of!(obs_source_frame, data) - 0usize];
    ["Offset of field: obs_source_frame::linesize"][::std::mem::offset_of!(obs_source_frame, linesize) - 64usize];
    ["Offset of field: obs_source_frame::width"][::std::mem::offset_of!(obs_source_frame, width) - 96usize];
    ["Offset of field: obs_source_frame::height"][::std::mem::offset_of!(obs_source_frame, height) - 100usize];
    ["Offset of field: obs_source_frame::timestamp"][::std::mem::offset_of!(obs_source_frame, timestamp) - 104usize];
    ["Offset of field: obs_source_frame::format"][::std::mem::offset_of!(obs_source_frame, format) - 112usize];
    ["Offset of field: obs_source_frame::color_matrix"]
        [::std::mem::offset_of!(obs_source_frame, color_matrix) - 116usize];
    ["Offset of field: obs_source_frame::full_range"][::std::mem::offset_of!(obs_source_frame, full_range) - 180usize];
    ["Offset of field: obs_source_frame::max_luminance"]
        [::std::mem::offset_of!(obs_source_frame, max_luminance) - 182usize];
    ["Offset of field: obs_source_frame::color_range_min"]
        [::std::mem::offset_of!(obs_source_frame, color_range_min) - 184usize];
    ["Offset of field: obs_source_frame::color_range_max"]
        [::std::mem::offset_of!(obs_source_frame, color_range_max) - 196usize];
    ["Offset of field: obs_source_frame::flip"][::std::mem::offset_of!(obs_source_frame, flip) - 208usize];
    ["Offset of field: obs_source_frame::flags"][::std::mem::offset_of!(obs_source_frame, flags) - 209usize];
    ["Offset of field: obs_source_frame::trc"][::std::mem::offset_of!(obs_source_frame, trc) - 210usize];
    ["Offset of field: obs_source_frame::refs"][::std::mem::offset_of!(obs_source_frame, refs) - 216usize];
    ["Offset of field: obs_source_frame::prev_frame"][::std::mem::offset_of!(obs_source_frame, prev_frame) - 224usize];
};
impl Default for obs_source_frame {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}
unsafe extern "C" {
    pub fn obs_source_output_video(source: *mut obs_source_t, frame: *const obs_source_frame);
}
unsafe extern "C" {
    pub fn os_gettime_ns() -> u64;
}
