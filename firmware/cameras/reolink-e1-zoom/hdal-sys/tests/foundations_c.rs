//! Compile the public C boundary for ARM; never execute HDAL or a device binary.
use std::io::Write;
use std::process::{Command, Stdio};

fn check(source: &str) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut child = Command::new(std::env::var_os("CLANG").unwrap_or_else(|| "clang".into()))
        .args([
            "--target=armv7-unknown-linux-gnueabihf",
            "-mcpu=cortex-a9",
            "-nostdlibinc",
            "-isystem",
        ])
        .arg(root.join("shim"))
        .arg("-I")
        .arg(root.join("vendor/hdal-include"))
        .args([
            "-std=c11",
            "-Werror=shift-sign-overflow",
            "-x",
            "c",
            "-fsyntax-only",
            "-",
        ])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("clang is required for the ARM header boundary tests");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(source.as_bytes())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn pixel_type_removes_only_the_bayer_arrangement_bits() {
    check(
        r#"
        #include "hd_type.h"
        _Static_assert(HD_VIDEO_PXLFMT_TYPE(0x410a0103u) == 0x410a0000u, "RAW10 type");
    "#,
    );
}

#[test]
fn compressed_raw_format_construction_has_no_signed_overflow() {
    check(
        r#"
        #include "hd_type.h"
        _Static_assert(HD_VIDEO_PXLFMT_MAKE_NRX(10, 1, HD_VIDEO_PIX_RGGB_B) == 0xf10a0103u, "NRX10");
    "#,
    );
}

#[test]
fn vendor_macroblock_format_is_a_self_contained_hdal_constant() {
    check(
        r#"
        #include "vendor/vendor_common.h"
        _Static_assert(VENDOR_VIDEO_PXLFMT_YUV420_MB3 == 0x510cd420u, "8x4 macroblock format");
    "#,
    );
}

#[test]
fn arm_foundations_match_recorded_scalar_and_descriptor_contracts() {
    check(
        r#"
        #include "hdal.h"
        #define SIZE(t, n) _Static_assert(sizeof(t) == (n), #t " size")
        #define OFFSET(t, f, n) _Static_assert(__builtin_offsetof(t, f) == (n), #t "." #f)
        SIZE(void *, 4); SIZE(UINTPTR, 4); SIZE(INTPTR, 4);
        SIZE(CHAR, 1); SIZE(INT8, 1); SIZE(UINT8, 1);
        SIZE(INT16, 2); SIZE(UINT16, 2);
        SIZE(INT32, 4); SIZE(UINT32, 4); SIZE(LONG, 4); SIZE(ULONG, 4);
        SIZE(BOOL, 4); SIZE(BOOLEAN, 4); SIZE(HD_RESULT, 4); SIZE(HD_VIDEO_PXLFMT, 4);
        SIZE(UINT64, 8); SIZE(INT64, 8); SIZE(DOUBLE, 8);
        _Static_assert((CHAR)-1 == 255 && (INT8)-1 < 0, "plain vs signed char");
        _Static_assert((BOOL)-1 > 0 && (BOOLEAN)-1 < 0 && (HD_RESULT)-1 < 0, "signedness");
        _Static_assert(_Alignof(UINT64) == 8 && _Alignof(DOUBLE) == 8, "AAPCS alignment");
        _Static_assert(__BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__, "little endian");
        _Static_assert(HD_VIDEO_FRC_RATIO(10, 30) == 0x000a001eu, "FRC does not reduce");
        _Static_assert(HD_VIDEO_FRC_RATIO(0x10001u, 0x10002u) == 0x00010002u, "FRC masks halves");
        _Static_assert(HD_COMMON_MEM_VB_INVALID_BLK == 0xffffffffu, "invalid block handle");
        SIZE(HD_COMMON_MEM_POOL_INFO, 24); SIZE(HD_COMMON_MEM_INIT_CONFIG, 3072);
        OFFSET(HD_COMMON_MEM_POOL_INFO, blk_size, 8);
        OFFSET(HD_COMMON_MEM_POOL_INFO, blk_cnt, 12);
        OFFSET(HD_COMMON_MEM_POOL_INFO, start_addr, 16);
        OFFSET(HD_COMMON_MEM_POOL_INFO, shared_pool, 20);
        SIZE(HD_VIDEO_BS_DATA, 12); SIZE(HD_VIDEOENC_BS, 144);
        OFFSET(HD_VIDEOENC_BS, pack_num, 12); OFFSET(HD_VIDEOENC_BS, timestamp, 16);
        OFFSET(HD_VIDEOENC_BS, video_pack, 32); OFFSET(HD_VIDEOENC_BS, blk, 136);
        OFFSET(HD_VIDEOENC_BS, motion_ratio, 140);
        _Static_assert(VENC_BS_PACK_MAX == 4 && HD_AUDIO_MAX_CH == 2, "fixed descriptor arrays");
        SIZE(HD_AUDIO_BS, 40); OFFSET(HD_AUDIO_BS, timestamp, 16);
        OFFSET(HD_AUDIO_BS, size, 24); OFFSET(HD_AUDIO_BS, phy_addr, 32);
        SIZE(HD_AUDIO_FRAME, 64); OFFSET(HD_AUDIO_FRAME, phy_addr, 16);
        OFFSET(HD_AUDIO_FRAME, count, 40); OFFSET(HD_AUDIO_FRAME, timestamp, 48);
        OFFSET(HD_AUDIO_FRAME, blk, 56);
        SIZE(HD_VIDEOENC_OUT, 44); SIZE(HD_VIDEOENC_OUT2, 44);
        OFFSET(HD_VIDEOENC_OUT, h26x, 4); OFFSET(HD_VIDEOENC_OUT, jpeg, 4);
        OFFSET(HD_VIDEOENC_OUT2, h26x, 4); OFFSET(HD_VIDEOENC_OUT2, jpeg, 4);
        #define SIGNATURE(fn, type) _Static_assert(__builtin_types_compatible_p(__typeof__(&(fn)), type), #fn)
        SIGNATURE(hd_common_mem_mmap, void *(*)(HD_COMMON_MEM_MEM_TYPE, UINTPTR, UINT32));
        SIGNATURE(hd_common_mem_munmap, HD_RESULT (*)(void *, unsigned int));
        SIGNATURE(hd_common_mem_flush_cache, HD_RESULT (*)(void *, unsigned int));
        SIGNATURE(hd_common_mem_alloc, HD_RESULT (*)(CHAR *, UINTPTR *, void **, UINT32, HD_COMMON_MEM_DDR_ID));
        SIGNATURE(hd_common_mem_free, HD_RESULT (*)(UINTPTR, void *));
    "#,
    );
}
