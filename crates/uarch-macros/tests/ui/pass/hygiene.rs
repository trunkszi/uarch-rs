use uarch_macros::{cache_line, register_pass};

#[register_pass]
#[derive(Clone, Copy)]
struct Recursive {
    value: u64,
    marker: core::marker::PhantomData<Self>,
}

const BYTES: usize = 32;
#[cache_line]
struct Exact {
    first: [u8; BYTES],
    second: [u8; BYTES],
}

fn proof<T: uarch::__private::NoPadding>() {}
fn main() {
    proof::<Recursive>();
    proof::<Exact>();
}
