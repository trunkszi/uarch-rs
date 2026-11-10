//! Higher-level seam tests cover this module without duplicate unit cases.

#[test]
fn shadowed_primitives_preserve_width() {
    #[allow(non_camel_case_types)]
    type u16 = u128;
    uarch_core::bitword! {
        pub struct Shadowed(u16) {
            [0] pub FLAG,
            [8..=15] pub HIGH: ::core::primitive::u8,
        }
    }
    assert_eq!(core::mem::size_of::<u16>(), 16);
    assert_eq!(core::mem::size_of::<Shadowed>(), 2);
    assert_eq!(<Shadowed as uarch_core::layout::RegisterLeaf>::LEAVES, 1);
    let mut word = Shadowed::new(0x005a).patch(Shadowed::HIGH.patch(0xa5));
    word |= Shadowed::FLAG;
    assert_eq!(word.get(Shadowed::HIGH), 0xa5);
    let raw: ::core::primitive::u16 = word.raw();
    assert_eq!(raw, 0xa55b);
}
