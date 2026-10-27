//! Camera scalar and constant contracts, independent of the host's C ABI.
use hdal_sys::*;

#[test]
fn plain_char_uses_the_arm_unsigned_range_on_every_host() {
    // ARM AAPCS plain char is unsigned; signed char remains a separate type.
    assert_eq!(CHAR::MAX as u16, 255);
    assert_eq!(INT8::MIN, -128);
}

#[test]
fn audio_decoder_input_twenty_has_its_own_port() {
    assert_eq!(HD_AUDIODEC_0_IN_20, 0x1300_1500);
    assert_ne!(HD_AUDIODEC_0_IN_20, HD_AUDIODEC_0_IN_10);
}
