//! Higher-level seam tests cover this module without duplicate unit cases.

#[test]
fn bitmap_methods_match_bit_oracle() {
    use uarch_core::bits::Bitmap64;
    const TOP: Option<u32> = Bitmap64::FULL.highest_one_index();
    assert_eq!(TOP, Some(63));
    let mut seed = 0x1234_4567_89ab_cdefu64;
    for _ in 0..128 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let map = Bitmap64::new(seed);
        let bits: Vec<_> = (0..64).filter(|bit| seed & (1u64 << bit) != 0).collect();
        assert_eq!(map.count_ones() as usize, bits.len());
        assert_eq!(map.is_empty(), bits.is_empty());
        assert_eq!(map.highest_one_index(), bits.last().copied());
        assert_eq!(
            map.lowest_zero_index(),
            (0..64).find(|bit| seed & (1u64 << bit) == 0)
        );
        for bit in 0..=65 {
            assert_eq!(
                map.next_one_from(bit),
                bits.iter().copied().find(|value| *value >= bit)
            );
            assert_eq!(
                map.prev_one_from(bit),
                bits.iter().copied().rfind(|value| *value <= bit)
            );
            if bit < 64 {
                assert_eq!(map.test(bit), seed & (1u64 << bit) != 0);
                assert_eq!(map.set(bit).raw(), seed | (1u64 << bit));
                assert_eq!(map.clear(bit).raw(), seed & !(1u64 << bit));
            }
        }
        let mut popped = map;
        for bit in bits.iter().rev() {
            assert_eq!(popped.pop_highest_one(), Some(*bit));
        }
        assert_eq!(popped.pop_highest_one(), None);
        assert_eq!(map.clear_lowest_one().raw(), seed & seed.wrapping_sub(1));
        assert_eq!(
            map.clear_highest_one().raw(),
            bits.last().map_or(seed, |bit| seed & !(1u64 << bit))
        );
        let other = Bitmap64::new(seed.rotate_left(7));
        assert_eq!(map.union(other), map | other);
        assert_eq!(map.intersection(other), map & other);
        assert_eq!(map.difference(other), map & !other);
        let mut assigned = map;
        assigned |= other;
        assert_eq!(assigned, map | other);
        assigned = map;
        assigned &= other;
        assert_eq!(assigned, map & other);
        assigned = map;
        assigned ^= other;
        assert_eq!(assigned, map ^ other);
    }
    assert_eq!(Bitmap64::EMPTY.highest_one_bit(), 0);
    assert_eq!(Bitmap64::FULL.lowest_zero_index(), None);
}

#[test]
fn bitmap_rejects_invalid_indices() {
    use uarch_core::bits::Bitmap64;
    for bit in [64, u32::MAX] {
        assert!(std::panic::catch_unwind(|| Bitmap64::EMPTY.test(bit)).is_err());
        assert!(std::panic::catch_unwind(|| Bitmap64::EMPTY.set(bit)).is_err());
        assert!(std::panic::catch_unwind(|| Bitmap64::EMPTY.clear(bit)).is_err());
    }
}
