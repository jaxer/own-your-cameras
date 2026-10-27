//! Raw bindings to the camera's `libhdal.so` (HDAL 2.00.000), generated from the vendored headers.
//!
//! The headers are HDAL 3.02; `abi/` and `HEADERS.md` say where the 2.00 library differs.
#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code
)]
#![allow(clippy::all, unsafe_op_in_unsafe_fn, deref_nullptr)]

/// C scalar types as the camera (ILP32) has them, on every host.
/// Pointers and bindgen's `usize`/`isize` aliases still follow the compilation target.
pub mod ctypes {
    pub use core::ffi::{c_int, c_schar, c_short, c_uchar, c_uint, c_ushort, c_void};
    // AAPCS plain char is unsigned, including when tests run on a signed-char host.
    pub type c_char = u8;
    pub type c_long = i32;
    pub type c_ulong = u32;
    pub type c_longlong = i64;
    pub type c_ulonglong = u64;
}

#[cfg(not(target_arch = "arm"))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
#[cfg(target_arch = "arm")]
include!(concat!(env!("OUT_DIR"), "/arm_bindings.rs"));

/// `(record name, size in bytes)` for the generated wrapper's structs, unions and record aliases,
/// as clang computes them for ARMv7. This is header evidence, not proof of the binary's ABI.
/// Host-independent, unlike `size_of`, which follows the host's pointer width.
pub mod header_sizes {
    include!(concat!(env!("OUT_DIR"), "/header_sizes.rs"));
}
