extern crate uarch as alternate;
use alternate::{Arch, Scalar};
use uarch_macros::{cache_line, isolated, multiversion, pattern, patch, register_pass};

#[register_pass(crate = "::alternate")]
#[derive(Clone, Copy)]
struct Small {
    value: u64,
}
#[cache_line(crate = "::alternate", lines = 1)]
struct Line {
    value: [u8; 64],
}
#[isolated(crate = "::alternate", full)]
struct Full {
    value: [u8; 128],
}
#[multiversion(crate = "::alternate")]
fn kernel<A: Arch>(arch: A) -> usize {
    let _ = arch;
    A::VECTOR_BYTES
}

alternate::bitword! { pub struct Word(u64) { [0] pub READY, [8..=11] pub STATE:u8, } }

fn proof<T: alternate::__private::NoPadding>() {}
fn main() {
    proof::<Small>();
    proof::<Line>();
    proof::<Full>();
    let _ = pattern!(crate = "::alternate", Word { READY & STATE == 2 });
    let _ = patch!(
        crate = "::alternate",
        Word {
            READY: true,
            STATE: 2
        }
    );
    // SAFETY: Scalar has no optional hardware requirements.
    assert_eq!(kernel(unsafe { Scalar::new_unchecked() }), 0);
}
