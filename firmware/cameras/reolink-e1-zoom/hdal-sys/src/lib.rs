//! Maintained declarations for the camera's `libhdal.so` (HDAL 2.00.000).
//!
//! `abi/` and `HEADERS.md` record the qualified interface and evidence limits.
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

include!("abi.rs");
#[cfg(target_arch = "arm")]
include!("arm_layout.rs");

/// Frozen ARM declaration sizes; historical evidence limits remain in abi/README.md.
pub mod header_sizes {
    include!("header_sizes.rs");
}
