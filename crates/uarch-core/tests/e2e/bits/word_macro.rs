use uarch_core::bits::{BitPatch, BitPattern};
use uarch_core::bitword;

bitword! {
    #[derive(Debug)]
    pub struct Meta(u64) {
        [0] pub READY,
        [1] pub ACTIVE,
        [2] pub URGENT,
        [8..=11] pub STATE: u8,
        [12..=15] pub OWNER: u8,
        [32..=63] pub SEQ: u32,
    }
}

#[test]
fn updates_preserve_other_fields() {
    const QUERY: BitPattern<Meta> = BitPattern::ANY
        .and_flag(Meta::READY)
        .and_not(Meta::URGENT)
        .and_eq(Meta::STATE, 3);
    const PATCH: BitPatch<Meta> = BitPatch::EMPTY
        .with_field(Meta::STATE, 3)
        .with_flag(Meta::READY, true);
    let mut meta = Meta::new(0x0000_0001_0000_f000).patch(PATCH);
    assert!(meta.matches(QUERY));
    assert_eq!(meta.get(Meta::STATE), 3);
    meta |= Meta::ACTIVE | Meta::URGENT;
    meta &= !(Meta::ACTIVE | Meta::URGENT);
    meta ^= Meta::URGENT;
    assert!(!meta.matches(QUERY));
    meta ^= Meta::URGENT | Meta::ACTIVE;
    meta |= Meta::URGENT;
    meta &= !Meta::URGENT;
    meta ^= Meta::ACTIVE;
    assert!(meta.matches(QUERY));
    meta = meta.wrapping_add_field(Meta::OWNER, 1);
    assert_eq!(meta.get(Meta::OWNER), 0);
    assert_eq!(meta.get(Meta::STATE), 3);
    meta = meta.wrapping_add_top_field(Meta::SEQ, u32::MAX as u64);
    assert_eq!(meta.get(Meta::SEQ), 0);
    assert_eq!(meta.raw(), 0x301);
}

#[test]
fn const_fields_cover_storage_widths() {
    macro_rules! check {
        ($name:ident,$storage:ident,$value:ident,$last:literal) => {{
            uarch_core::bitword! { struct $name($storage) { [0] FLAG, [1..=$last] VALUE:$value } }
            const WORD: $name = $name::new(0).patch($name::VALUE.patch(3));
            const RAW: u64 = $name::VALUE.extract_raw(WORD.raw() as u64);
            const FIELD: $value = WORD.get($name::VALUE);
            const FLAG: bool = WORD.test($name::FLAG);
            const QUERY: uarch_core::bits::BitPattern<$name> = $name::VALUE.eq(3);
            assert_eq!(FIELD, 3);
            assert_eq!(RAW, 3);
            const {
                assert!(!FLAG);
            }
            assert!(WORD.matches(QUERY));
            assert!($name::VALUE.try_patch(3).is_ok());
        }};
    }
    check!(Byte, u8, u8, 7);
    check!(Half, u16, u16, 15);
    check!(Word, u32, u32, 31);
    check!(Double, u64, u64, 63);
}

#[test]
fn field_queries_require_complete_match() {
    const WORD: Meta = Meta::new(0).patch(Meta::STATE.patch(2));
    const MATCHED: bool = WORD.matches(Meta::STATE.eq(3));
    const {
        assert!(!MATCHED);
    }
    assert!(WORD.matches(Meta::STATE.eq(2)));
}
