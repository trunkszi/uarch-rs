use uarch_arch::{Arch, Scalar};

#[test]
fn packed_bits_roundtrip() {
    let arch = Scalar::new();
    assert_eq!(arch.bit_gather(0b1010_1101, 0b1111_0000), 0b1010);
    assert_eq!(arch.bit_scatter(0b1101, 0b1010_1010), 0b1010_0010);
    for val in [
        0,
        1,
        2,
        0x0123_4567_89ab_cdef,
        0xdead_beef_feed_face,
        u64::MAX,
    ] {
        for mask in [
            0,
            1,
            1 << 63,
            0xffff,
            0x5555_5555_5555_5555,
            0x8000_0000_0000_0001,
            u64::MAX,
        ] {
            assert_eq!(
                arch.bit_scatter(arch.bit_gather(val, mask), mask),
                val & mask
            );
        }
    }
}
