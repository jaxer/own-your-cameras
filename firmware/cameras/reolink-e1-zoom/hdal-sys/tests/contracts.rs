//! Maintained declaration checks; historical layout literals remain separate.
use hdal_sys::*;

// Type-check the memory boundary without invoking or linking an SDK operation.
const _: unsafe extern "C" fn(HD_COMMON_MEM_MEM_TYPE, UINTPTR, UINT32) -> *mut VOID =
    hd_common_mem_mmap;
const _: unsafe extern "C" fn(*mut VOID, u32) -> HD_RESULT = hd_common_mem_munmap;
const _: unsafe extern "C" fn(*mut VOID, u32) -> HD_RESULT = hd_common_mem_flush_cache;
const _: unsafe extern "C" fn(
    *mut CHAR,
    *mut UINTPTR,
    *mut *mut VOID,
    UINT32,
    HD_COMMON_MEM_DDR_ID,
) -> HD_RESULT = hd_common_mem_alloc;
const _: unsafe extern "C" fn(UINTPTR, *mut VOID) -> HD_RESULT = hd_common_mem_free;

#[test]
fn maintained_records_retain_frozen_arm_layout_checks() {
    let declarations = include_str!("../src/abi.rs");
    let layouts = include_str!("../src/arm_layout.rs");
    for line in declarations.lines() {
        let Some(rest) = line
            .strip_prefix("pub struct ")
            .or_else(|| line.strip_prefix("pub union "))
        else {
            continue;
        };
        let name = rest.split_whitespace().next().unwrap();
        if name.contains('<') {
            continue;
        } // Generic bitfield storage has no concrete ABI.
        assert!(
            layouts.contains(&format!("Size of {name}\"")),
            "missing frozen layout: {name}"
        );
        assert!(
            layouts.contains(&format!("Alignment of {name}\"")),
            "missing frozen alignment: {name}"
        );
    }
}

#[test]
fn scalar_contracts_do_not_follow_the_hosts_long_width() {
    use core::mem::size_of;
    assert_eq!(size_of::<LONG>(), 4);
    assert_eq!(size_of::<ULONG>(), 4);
    assert_eq!(size_of::<HD_RESULT>(), 4);
    assert_eq!(size_of::<UINT64>(), 8);
    assert_eq!(size_of::<DOUBLE>(), 8);
    assert_eq!(BOOL::MAX, u32::MAX);
    assert_eq!(BOOLEAN::MIN, i32::MIN);
}
