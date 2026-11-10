use uarch_core::bits::Bitmap64;

#[test]
fn low_masks_cover_boundaries() {
    assert_eq!(Bitmap64::low_mask(0), 0);
    assert_eq!(Bitmap64::low_mask(1), 1);
    assert_eq!(Bitmap64::low_mask(63), 0x7fff_ffff_ffff_ffff);
    assert_eq!(Bitmap64::low_mask(64), u64::MAX);
    assert_eq!(Bitmap64::low_mask(u32::MAX), u64::MAX);
}
