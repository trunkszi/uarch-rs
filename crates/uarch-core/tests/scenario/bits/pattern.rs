use uarch_core::bits::{BitError, BitField, BitFlag, BitPatch, BitPattern};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct Word(u64);
const READY: BitFlag<Word> = BitFlag::new(0);
const FIELD: BitField<Word, u8, 1, 3> = BitField::new();

#[test]
fn overlaps_require_equal_values() {
    let ready = BitPattern::ANY.and_flag(READY);
    let clear = BitPattern::ANY.and_not(READY);
    assert_eq!(ready.try_and(clear), Err(BitError::Conflict));
    assert_eq!(ready & READY, ready);
    assert_eq!((READY & !BitFlag::new(4)).expected(), 1);
    assert_eq!(FIELD.eq(3).try_and(FIELD.eq(2)), Err(BitError::Conflict));
    let same = FIELD.patch(3).try_and(FIELD.patch(3));
    assert_eq!(same, Ok(FIELD.patch(3)));
    assert_eq!(
        FIELD.patch(3).try_and(FIELD.patch(2)),
        Err(BitError::Conflict)
    );
    assert_eq!(FIELD.patch(3).then(FIELD.patch(2)).apply(0xf1), 0xf5);
    const PATCH: BitPatch<Word> = BitPatch::EMPTY.with_flag(READY, true).with_field(FIELD, 3);
    assert_eq!(PATCH.apply(0), 7);
}
