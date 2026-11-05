#![deny(warnings)]
use uarch_macros::{pattern, patch};

uarch::bitword! {
    pub struct Word(u64) {
        [0] pub READY,
        [1] pub BUSY,
        [8..=11] pub STATE: u8,
    }
}

const STATE: u64 = 3;
struct Constants<T, U>(core::marker::PhantomData<(T, U)>);
impl<T, U> Constants<T, U> {
    const VALUE: u64 = 3;
}

fn main() {
    let _ = pattern!(crate::Word { READY });
    let _ = patch!(crate::Word {
        STATE: Constants::<u8, u16>::VALUE
    });
    let pattern = pattern!(Word { READY & !BUSY & STATE == (STATE & 7) });
    assert_eq!(pattern.mask(), 0xf03);
    assert_eq!(pattern.expected(), 0x301);
    let patch = patch!(Word {
        READY: true,
        BUSY: false,
        STATE: STATE,
    });
    assert_eq!(Word::new(0xffff).patch(patch).raw(), 0xf3fd);
    let _ = pattern!(crate = "::uarch", Word { STATE == STATE & READY });
    let _ = patch!(
        crate = "::uarch",
        Word {
            READY: (true),
            STATE: (STATE | 1)
        }
    );
}
