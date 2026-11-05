use uarch_core::bits::{BitField, BitFlag, BitPatch, BitPattern};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct Word(u64);
const FLAG: BitFlag<Word> = BitFlag::new(3);
const FIELD: BitField<Word, u64, 60, 63> = BitField::new();

#[test]
fn violations_panic_in_all_profiles() {
    let overflow = std::hint::black_box(16);
    assert!(std::panic::catch_unwind(|| FIELD.eq(overflow)).is_err());
    assert!(std::panic::catch_unwind(|| FIELD.patch(overflow)).is_err());
    assert!(std::panic::catch_unwind(|| FLAG & !FLAG).is_err());
    assert!(std::panic::catch_unwind(|| FIELD.eq(1) & FIELD.eq(2)).is_err());
    assert!(std::panic::catch_unwind(|| FIELD.patch(1).and(FIELD.patch(2))).is_err());
    assert!(std::panic::catch_unwind(|| BitPattern::ANY.and_eq(FIELD, overflow)).is_err());
    assert!(std::panic::catch_unwind(|| BitPatch::EMPTY.with_field(FIELD, overflow)).is_err());
    assert!(
        std::panic::catch_unwind(|| BitPatch::EMPTY.with_flag(FLAG, true).with_flag(FLAG, false))
            .is_err()
    );
}
