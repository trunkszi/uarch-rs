use uarch::prelude::*;
bitword! { struct Tiny(u8) { [0] pub READY, [3..=4] pub MODE: u8, [7] pub TOP } }
bitword! { struct Medium(u16) { [0] pub READY, [5..=8] pub MODE: u8, [15] pub TOP } }
bitword! { struct Wide(u32) { [0] pub READY, [28..=31] pub MODE: u8 } }
bitword! { struct Huge(u64) { [0] pub READY, [60..=63] pub MODE: u8 } }
bitword! { struct Empty(u64) {} }
bitword! { struct Full(u64) { [0..=63] pub VALUE: u64 } }

#[test]
fn declared_masks_cover_all_widths() {
    assert_eq!(Tiny::USED_MASK, 0x99);
    assert_eq!(Medium::USED_MASK, 0x81e1);
    assert_eq!(Wide::USED_MASK, 0xf000_0001);
    assert_eq!(Huge::USED_MASK, 0xf000_0000_0000_0001);
    assert_eq!(Empty::USED_MASK, 0);
    assert_eq!(Full::USED_MASK, u64::MAX);
    const ACCEPTED: [u8; 16] = [
        0, 1, 8, 9, 16, 17, 24, 25, 128, 129, 136, 137, 144, 145, 152, 153,
    ];
    for raw in 0..=u8::MAX {
        assert_eq!(Tiny::try_from_raw(raw).is_some(), ACCEPTED.contains(&raw));
        assert_eq!(
            Tiny::new(raw).has_undeclared_bits(),
            !ACCEPTED.contains(&raw)
        );
    }
    for bit in 0..64 {
        let raw = 1u64 << bit;
        assert_eq!(Huge::try_from_raw(raw).is_some(), bit == 0 || bit >= 60);
        assert_eq!(
            Huge::new(raw).has_undeclared_bits(),
            !(bit == 0 || bit >= 60)
        );
        assert!(Empty::try_from_raw(raw).is_none());
        assert!(Full::try_from_raw(raw).is_some());
    }
}

#[test]
fn raw_words_preserve_declared_bits() {
    for raw in 0..=u16::MAX {
        let valid = raw & !0x81e1 == 0;
        assert_eq!(
            Medium::try_from_raw(raw).map(Medium::raw),
            valid.then_some(raw)
        );
        assert_eq!(Medium::new(raw).has_undeclared_bits(), !valid);
    }
    for mode in 0..16 {
        for ready in 0..2 {
            let raw = (mode << 28) | ready;
            assert_eq!(Wide::try_from_raw(raw).map(Wide::raw), Some(raw));
            assert!(!Wide::new(raw).has_undeclared_bits());
            let raw = (u64::from(mode) << 60) | u64::from(ready);
            assert_eq!(Huge::try_from_raw(raw).map(Huge::raw), Some(raw));
        }
    }
    for bit in 1..28 {
        assert!(Wide::try_from_raw(1 << bit).is_none());
        assert!(Wide::new(1 << bit).has_undeclared_bits());
    }
    assert_eq!(Empty::try_from_raw(0).map(Empty::raw), Some(0));
    assert!(!Empty::new(0).has_undeclared_bits());
    assert_eq!(Full::try_from_raw(u64::MAX).map(Full::raw), Some(u64::MAX));
    assert!(!Full::new(u64::MAX).has_undeclared_bits());
}

#[test]
fn raw_patch_checks_field_boundaries() {
    use uarch::bits::BitError;
    const PATCHED: Tiny = Tiny::new(0xff).patch_raw(Tiny::MODE, 0);
    const CHECKED: Result<Tiny, BitError> = Tiny::new(0x81).try_patch(Tiny::MODE, 3);
    const OVERFLOW: Result<Tiny, BitError> = Tiny::new(0).try_patch(Tiny::MODE, 4);
    assert_eq!(PATCHED.raw(), 0xe7);
    assert_eq!(CHECKED.map(Tiny::raw), Ok(0x99));
    assert!(matches!(OVERFLOW, Err(BitError::ValueOverflow)));
    assert_eq!(Tiny::MODE.max_raw(), 3);
    assert_eq!(Full::VALUE.max_raw(), u64::MAX);
    for value in 0..=3 {
        assert_eq!(
            Tiny::new(0xff).try_patch(Tiny::MODE, value).map(Tiny::raw),
            Ok(0xe7 | (value as u8) << 3)
        );
    }
    for value in [4, 255, u64::MAX] {
        assert!(matches!(
            Tiny::new(0).try_patch(Tiny::MODE, value),
            Err(BitError::ValueOverflow)
        ));
    }
    assert_eq!(Medium::new(0xffff).patch_raw(Medium::MODE, 0).raw(), 0xfe1f);
    assert_eq!(Wide::new(0).patch_raw(Wide::MODE, 15).raw(), 0xf000_0000);
    assert_eq!(
        Huge::new(u64::MAX).patch_raw(Huge::MODE, 0).raw(),
        0x0fff_ffff_ffff_ffff
    );
    assert_eq!(
        Full::new(0).try_patch(Full::VALUE, u64::MAX).map(Full::raw),
        Ok(u64::MAX)
    );
    assert_eq!(Full::new(1).patch_raw(Full::VALUE, 0).raw(), 0);
}

#[test]
#[should_panic(expected = "raw_value <= field.max_raw()")]
fn raw_patch_panics_on_overflow() {
    let _ = Tiny::new(0).patch_raw(Tiny::MODE, 4);
}

#[test]
fn fnv_vectors_and_incremental_hashes() {
    use uarch::hash::{FNV1A64_OFFSET_BASIS, FNV1A64_PRIME};
    assert_eq!(FNV1A64_OFFSET_BASIS, 0xcbf2_9ce4_8422_2325);
    assert_eq!(FNV1A64_PRIME, 0x100_0000_01b3);
    const VECTORS: [(&[u8], u64); 4] = [
        (b"", 0xcbf2_9ce4_8422_2325),
        (b"a", 0xaf63_dc4c_8601_ec8c),
        (b"foobar", 0x8594_4171_f739_67e8),
        (b"BTCUSDT", 0x3e640dbff523962c),
    ];
    for (bytes, expected) in VECTORS {
        assert_eq!(fnv1a64(bytes), expected);
        for split in 0..=bytes.len() {
            assert_eq!(
                fnv1a64_extend(fnv1a64(&bytes[..split]), &bytes[split..]),
                expected
            );
        }
    }
    assert_eq!(fnv1a64_extend(42, b""), 42);
    assert_eq!(
        fnv1a64_extend(42, &[0xff]),
        (42u64 ^ 255).wrapping_mul(FNV1A64_PRIME)
    );
    const HASH: u64 = fnv1a64(b"foobar");
    assert_eq!(HASH, 0x8594_4171_f739_67e8);
}

#[test]
fn fixed_ascii_checked_and_layout() {
    assert_layout!(FixedAscii<0>, size = 0, align = 1);
    assert_layout!(FixedAscii<8>, size = 8, align = 1);
    assert_eq!(core::mem::offset_of!(FixedAscii<8>, 0), 0);
    assert_layout!(FixedAscii<32>, size = 32, align = 1);
    fn no_padding<T: NoPadding>() {}
    no_padding::<FixedAscii<32>>();
    const CHECKED: Option<FixedAscii<4>> = FixedAscii::try_from_str("abcd");
    assert_eq!(CHECKED.map(|s| s.0), Some(*b"abcd"));
    assert_eq!(
        FixedAscii::<4>::try_from_bytes(b"a").map(|s| s.0),
        Some([b'a', 0, 0, 0])
    );
    assert!(FixedAscii::<4>::try_from_str("abcde").is_none());
    assert!(FixedAscii::<4>::try_from_str("a\0b").is_none());
    assert!(FixedAscii::<4>::try_from_str("é").is_none());
    assert!(FixedAscii::<4>::try_from_bytes(&[0x80]).is_none());
    assert_eq!(
        FixedAscii::<1>::try_from_bytes(&[0x7f]).map(|s| s.0),
        Some([0x7f])
    );
    assert_eq!(FixedAscii::<0>::try_from_str(""), Some(FixedAscii::empty()));
    assert!(FixedAscii::<0>::try_from_str("a").is_none());
    assert_eq!(FixedAscii::<4>::default(), FixedAscii::empty());
}

#[test]
fn fixed_ascii_truncation_and_views() {
    const TEXT: FixedAscii<32> = FixedAscii::from_str_truncating("BTCUSDT");
    const HASH: u64 = TEXT.fnv1a64();
    assert_eq!(HASH, fnv1a64(b"BTCUSDT"));
    assert_eq!(TEXT.len(), 7);
    assert!(!TEXT.is_empty());
    assert_eq!(TEXT.as_str(), "BTCUSDT");
    assert_eq!(TEXT.as_trimmed_bytes(), b"BTCUSDT");
    assert_eq!(TEXT.as_bytes().len(), 32);
    let bytes: &[u8] = TEXT.as_ref();
    let string: &str = TEXT.as_ref();
    assert_eq!(bytes, TEXT.as_bytes());
    assert_eq!(string, "BTCUSDT");
    assert!(TEXT == "BTCUSDT");
    assert!(TEXT == *"BTCUSDT");
    assert!(TEXT != "BTC");
    assert_eq!(format!("{TEXT}"), "BTCUSDT");
    assert_eq!(format!("{TEXT:?}"), "FixedAscii(\"BTCUSDT\")");
    for (input, expected) in [
        (b"abcde".as_slice(), b"abcd".as_slice()),
        (b"ab\0cd", b"ab"),
        (b"ab\xffcd", b"ab"),
        (b"\xffab", b""),
        (b"", b""),
    ] {
        let text = FixedAscii::<4>::from_bytes_truncating(input);
        assert_eq!(text.as_trimmed_bytes(), expected);
        assert_eq!(text.fnv1a64(), fnv1a64(expected));
        assert!(text.as_bytes()[text.len()..].iter().all(|b| *b == 0));
    }
    assert!(FixedAscii::<0>::from_str_truncating("abc").is_empty());
    assert_eq!(FixedAscii::<0>::empty().fnv1a64(), fnv1a64(b""));
    assert_eq!(FixedAscii::<4>::from_str_truncating("abé").as_str(), "ab");
}

#[test]
fn public_storage_remains_utf8_safe() {
    let mut text = FixedAscii::<4>([0xff, 1, 2, 3]);
    assert_eq!(text.as_str(), "");
    assert_eq!(text.len(), 4);
    assert_eq!(text.fnv1a64(), fnv1a64(&[0xff, 1, 2, 3]));
    text.0 = [b'a', 0, 0xff, 0xff];
    assert_eq!(text.as_str(), "a");
    assert_eq!(text.fnv1a64(), fnv1a64(b"a"));
    text.0 = [0xc3, 0xa9, 0, 0];
    assert_eq!(text.as_str(), "é");
}

#[unsafe(no_mangle)]
pub fn primitive_raw_probe(raw: u64) -> bool {
    Huge::try_from_raw(raw).is_some()
}

#[unsafe(no_mangle)]
pub fn primitive_patch_probe(raw: u64, value: u64) -> Option<u64> {
    Huge::new(raw)
        .try_patch(Huge::MODE, value)
        .ok()
        .map(Huge::raw)
}

#[unsafe(no_mangle)]
pub fn primitive_hash_probe(bytes: &[u8]) -> u64 {
    fnv1a64(bytes)
}

#[unsafe(no_mangle)]
pub fn primitive_text_probe(bytes: &[u8]) -> Option<FixedAscii<8>> {
    FixedAscii::try_from_bytes(bytes)
}

#[unsafe(no_mangle)]
pub fn primitive_text_hash_probe(text: &FixedAscii<32>) -> u64 {
    text.fnv1a64()
}

#[unsafe(no_mangle)]
pub fn primitive_text_str_probe(text: &FixedAscii<32>) -> &str {
    text.as_str()
}
