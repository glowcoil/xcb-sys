#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_imports)]

use std::ffi::{c_char, c_int, c_uint, c_void};

use xproto::*;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub const X_PROTOCOL: u32 = 11;
pub const X_PROTOCOL_REVISION: u32 = 0;
pub const X_TCP_PORT: u32 = 6000;
pub const XCB_CONN_ERROR: u32 = 1;
pub const XCB_CONN_CLOSED_EXT_NOTSUPPORTED: u32 = 2;
pub const XCB_CONN_CLOSED_MEM_INSUFFICIENT: u32 = 3;
pub const XCB_CONN_CLOSED_REQ_LEN_EXCEED: u32 = 4;
pub const XCB_CONN_CLOSED_PARSE_ERR: u32 = 5;
pub const XCB_CONN_CLOSED_INVALID_SCREEN: u32 = 6;
pub const XCB_CONN_CLOSED_FDPASSING_FAILED: u32 = 7;
pub const XCB_NONE: u32 = 0;
pub const XCB_COPY_FROM_PARENT: u32 = 0;
pub const XCB_CURRENT_TIME: u32 = 0;
pub const XCB_NO_SYMBOL: u32 = 0;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct xcb_connection_t {
    _data: [u8; 0],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct xcb_generic_iterator_t {
    pub data: *mut c_void,
    pub rem: c_int,
    pub index: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct xcb_generic_reply_t {
    pub response_type: u8,
    pub pad0: u8,
    pub sequence: u16,
    pub length: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct xcb_generic_event_t {
    pub response_type: u8,
    pub pad0: u8,
    pub sequence: u16,
    pub pad: [u32; 7],
    pub full_sequence: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct xcb_raw_generic_event_t {
    pub response_type: u8,
    pub pad0: u8,
    pub sequence: u16,
    pub pad: [u32; 7],
}
#[repr(C)]
#[derive(Copy, Clone)]
pub struct xcb_ge_event_t {
    pub response_type: u8,
    pub pad0: u8,
    pub sequence: u16,
    pub length: u32,
    pub event_type: u16,
    pub pad1: u16,
    pub pad: [u32; 5],
    pub full_sequence: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct xcb_generic_error_t {
    pub response_type: u8,
    pub error_code: u8,
    pub sequence: u16,
    pub resource_id: u32,
    pub minor_code: u16,
    pub major_code: u8,
    pub pad0: u8,
    pub pad: [u32; 5],
    pub full_sequence: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct xcb_void_cookie_t {
    pub sequence: c_uint,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct xcb_auth_info_t {
    pub namelen: c_int,
    pub name: *mut c_char,
    pub datalen: c_int,
    pub data: *mut c_char,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct xcb_special_event_t {
    _data: [u8; 0],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct xcb_extension_t {
    pub name: *const c_char,
    pub global_id: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct xcb_protocol_request_t {
    pub count: usize,
    pub ext: *mut xcb_extension_t,
    pub opcode: u8,
    pub isvoid: u8,
}

pub type xcb_send_request_flags_t = u32;

pub const XCB_REQUEST_CHECKED: xcb_send_request_flags_t = 1 << 0;
pub const XCB_REQUEST_RAW: xcb_send_request_flags_t = 1 << 1;
pub const XCB_REQUEST_DISCARD_REPLY: xcb_send_request_flags_t = 1 << 2;
pub const XCB_REQUEST_REPLY_FDS: xcb_send_request_flags_t = 1 << 3;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct iovec {
    iov_base: *mut c_void,
    iov_len: usize,
}

extern "C" {
    pub fn xcb_flush(c: *mut xcb_connection_t) -> c_int;
    pub fn xcb_get_maximum_request_length(c: *mut xcb_connection_t) -> u32;
    pub fn xcb_prefetch_maximum_request_length(c: *mut xcb_connection_t);
    pub fn xcb_wait_for_event(c: *mut xcb_connection_t) -> *mut xcb_generic_event_t;
    pub fn xcb_poll_for_event(c: *mut xcb_connection_t) -> *mut xcb_generic_event_t;
    pub fn xcb_poll_for_queued_event(c: *mut xcb_connection_t) -> *mut xcb_generic_event_t;
    pub fn xcb_poll_for_special_event(
        c: *mut xcb_connection_t,
        se: *mut xcb_special_event_t,
    ) -> *mut xcb_generic_event_t;
    pub fn xcb_wait_for_special_event(
        c: *mut xcb_connection_t,
        se: *mut xcb_special_event_t,
    ) -> *mut xcb_generic_event_t;
    pub fn xcb_register_for_special_xge(
        c: *mut xcb_connection_t,
        ext: *mut xcb_extension_t,
        eid: u32,
        stamp: *mut u32,
    ) -> *mut xcb_special_event_t;
    pub fn xcb_unregister_for_special_event(c: *mut xcb_connection_t, se: *mut xcb_special_event_t);
    pub fn xcb_request_check(
        c: *mut xcb_connection_t,
        cookie: xcb_void_cookie_t,
    ) -> *mut xcb_generic_error_t;
    pub fn xcb_discard_reply(c: *mut xcb_connection_t, sequence: c_uint);
    pub fn xcb_discard_reply64(c: *mut xcb_connection_t, sequence: u64);
    pub fn xcb_get_extension_data(
        c: *mut xcb_connection_t,
        ext: *mut xcb_extension_t,
    ) -> *const xcb_query_extension_reply_t;
    pub fn xcb_prefetch_extension_data(c: *mut xcb_connection_t, ext: *mut xcb_extension_t);
    pub fn xcb_get_setup(c: *mut xcb_connection_t) -> *const xcb_setup_t;
    pub fn xcb_get_file_descriptor(c: *mut xcb_connection_t) -> c_int;
    pub fn xcb_connection_has_error(c: *mut xcb_connection_t) -> c_int;
    pub fn xcb_connect_to_fd(fd: c_int, auth_info: *mut xcb_auth_info_t) -> *mut xcb_connection_t;
    pub fn xcb_disconnect(c: *mut xcb_connection_t);
    pub fn xcb_parse_display(
        name: *const c_char,
        host: *mut *mut c_char,
        display: *mut c_int,
        screen: *mut c_int,
    ) -> c_int;
    pub fn xcb_connect(displayname: *const c_char, screenp: *mut c_int) -> *mut xcb_connection_t;
    pub fn xcb_connect_to_display_with_auth_info(
        display: *const c_char,
        auth: *mut xcb_auth_info_t,
        screen: *mut c_int,
    ) -> *mut xcb_connection_t;
    pub fn xcb_generate_id(c: *mut xcb_connection_t) -> u32;
    pub fn xcb_send_request(
        c: *mut xcb_connection_t,
        flags: c_int,
        vector: *mut iovec,
        request: *const xcb_protocol_request_t,
    ) -> c_uint;
    pub fn xcb_send_request_with_fds(
        c: *mut xcb_connection_t,
        flags: c_int,
        vector: *mut iovec,
        request: *const xcb_protocol_request_t,
        num_fds: c_uint,
        fds: *mut c_int,
    ) -> c_uint;
    pub fn xcb_send_request64(
        c: *mut xcb_connection_t,
        flags: c_int,
        vector: *mut iovec,
        request: *const xcb_protocol_request_t,
    ) -> u64;
    pub fn xcb_send_request_with_fds64(
        c: *mut xcb_connection_t,
        flags: c_int,
        vector: *mut iovec,
        request: *const xcb_protocol_request_t,
        num_fds: c_uint,
        fds: *mut c_int,
    ) -> u64;
    pub fn xcb_send_fd(c: *mut xcb_connection_t, fd: c_int) -> c_void;
    pub fn xcb_take_socket(
        c: *mut xcb_connection_t,
        return_socket: extern "C" fn(closure: *mut c_void),
        closure: *mut c_void,
        flags: c_int,
        sent: *mut u64,
    ) -> c_int;
    pub fn xcb_writev(
        c: *mut xcb_connection_t,
        vector: *mut iovec,
        count: c_int,
        requests: u64,
    ) -> c_int;
    pub fn xcb_wait_for_reply(
        c: *mut xcb_connection_t,
        request: c_uint,
        e: *mut *mut xcb_generic_error_t,
    ) -> *mut c_void;
    pub fn xcb_wait_for_reply64(
        c: *mut xcb_connection_t,
        request: u64,
        e: *mut *mut xcb_generic_error_t,
    ) -> *mut c_void;
    pub fn xcb_poll_for_reply(
        c: *mut xcb_connection_t,
        request: c_uint,
        reply: *mut *mut c_void,
        error: *mut *mut xcb_generic_error_t,
    ) -> c_int;
    pub fn xcb_poll_for_reply64(
        c: *mut xcb_connection_t,
        request: u64,
        reply: *mut *mut c_void,
        error: *mut *mut xcb_generic_error_t,
    ) -> c_int;
    pub fn xcb_get_reply_fds(
        c: *mut xcb_connection_t,
        reply: *mut c_void,
        replylen: usize,
    ) -> *mut c_int;
    pub fn xcb_popcount(mask: u32) -> c_int;
    pub fn xcb_sumof(list: *mut u8, len: c_int) -> c_int;
}
