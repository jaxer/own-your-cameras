//! Test the bindgen-output/evidence boundary, including failed extraction.
#[path = "../layout.rs"]
mod layout;

#[test]
fn empty_or_incomplete_layout_evidence_is_rejected() {
    let complete = r#"
        pub struct _HD_DIM { pub w: u32, pub h: u32 }
        const _: () = {
            ["Size of _HD_DIM"][::core::mem::size_of::<_HD_DIM>() - 8usize];
        };
        pub type HD_DIM = _HD_DIM;
    "#;
    assert_eq!(layout::extract_sizes(complete).unwrap()["HD_DIM"], "8");
    for input in [
        String::new(),
        complete.replace("Size of", "Changed bindgen format"),
        format!("{complete}\npub union _HD_MISSING {{ pub word: u32 }}"),
    ] {
        assert!(
            layout::extract_sizes(&input).is_err(),
            "accepted incomplete evidence: {input}"
        );
    }
}

#[test]
fn arm_bindings_carry_compile_time_layout_checks() {
    let arm = std::fs::read_to_string(concat!(env!("OUT_DIR"), "/arm_bindings.rs"))
        .expect("ARM bindings must carry clang's layout checks into rustc");
    for assertion in [
        "Size of _HD_COMMON_MEM_POOL_INFO",
        "Alignment of _HD_COMMON_MEM_POOL_INFO",
        "Offset of field: _HD_COMMON_MEM_POOL_INFO::blk_size",
        "Offset of field: _HD_VIDEOENC_BS::timestamp",
        "Offset of field: _HD_AUDIO_FRAME::timestamp",
        "Size of _HD_VIDEOENC_OUT__bindgen_ty_1",
    ] {
        assert!(
            arm.contains(assertion),
            "missing ARM assertion: {assertion}"
        );
    }
}

#[test]
fn record_aliases_resolve_independently_of_order_and_line_wrapping() {
    let input = r#"
        pub struct _HD_DIM { pub w: u32, pub h: u32 }
        ["Size of _HD_DIM"][::core::mem::size_of::<_HD_DIM>() - 8usize];
        pub type HD_DIM_ALIAS = HD_DIM;
        pub type HD_DIM =
            _HD_DIM;
    "#;
    let sizes = layout::extract_sizes(input).unwrap();
    assert_eq!(sizes.get("HD_DIM_ALIAS").map(String::as_str), Some("8"));
}

#[test]
fn ambiguous_size_claims_are_rejected() {
    for input in [
        r#"["Size of A"][::core::mem::size_of::<B>() - 8usize];"#,
        r#"["Size of A"][::core::mem::size_of::<A>() - 8usize];
            ["Size of A"][::core::mem::size_of::<A>() - 16usize];"#,
    ] {
        assert!(layout::extract_sizes(input).is_err());
    }
}
