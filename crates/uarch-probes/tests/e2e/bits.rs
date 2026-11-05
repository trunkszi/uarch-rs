#[test]
fn patch_and_bitmap_match_reference() {
    for raw in [0, 1, 2, 1 << 63, u64::MAX, 0x1234_5678_9abc_def0] {
        assert_eq!(
            uarch_probes::bits::bitpatch_apply(raw),
            (raw & !0x701) | 0x501
        );
        let (rest, bit) = uarch_probes::bits::bitmap64_pop_lowest(raw);
        assert_eq!(rest, raw & raw.wrapping_sub(1));
        assert_eq!(bit, raw.trailing_zeros());
        let bitmap = uarch_core::bits::Bitmap64::new(raw);
        let highest = (0..64).rev().find(|bit| raw & (1 << bit) != 0);
        let expected = highest.map_or(0, |bit| 1 << bit);
        assert_eq!(bitmap.highest_one_bit(), expected);
        assert_eq!(bitmap.clear_highest_one().raw(), raw ^ expected);
    }
}
