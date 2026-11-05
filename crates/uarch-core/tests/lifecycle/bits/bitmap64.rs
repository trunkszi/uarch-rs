use uarch_core::bits::Bitmap64;

#[test]
fn iteration_drains_in_order() {
    let mut map = Bitmap64::new(0x8000_0000_0000_0081);
    assert_eq!(map.lowest_one_index(), Some(0));
    assert_eq!(map.lowest_one_bit(), 1);
    let mut iter_ones = map.into_iter();
    assert_eq!(iter_ones.size_hint(), (3, Some(3)));
    assert_eq!(iter_ones.next(), Some(0));
    assert_eq!(iter_ones.len(), 2);
    assert_eq!(iter_ones.collect::<Vec<_>>(), [7, 63]);
    assert_eq!(map.pop_lowest_one(), Some(0));
    assert_eq!(map.pop_lowest_one(), Some(7));
    assert_eq!(map.lowest_one_bit(), 1 << 63);
    assert_eq!(map.pop_lowest_one(), Some(63));
    assert_eq!(map.lowest_one_index(), None);
    assert_eq!(map.pop_lowest_one(), None);
    assert_eq!(map.lowest_one_bit(), 0);
    let mut empty = map.into_iter();
    assert_eq!(empty.next(), None);
    assert_eq!(empty.next(), None);
}
