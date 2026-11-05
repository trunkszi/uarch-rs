use uarch_macros::register_pass;

#[register_pass]
#[derive(Clone, Copy)]
struct Pair {
    left: u64,
    right: u64,
}

fn requires_padding_proof<T: uarch::__private::NoPadding>() {}

fn main() {
    requires_padding_proof::<Pair>();
    let p = Pair { left: 2, right: 3 };
    assert_eq!(p.left + p.right, 5);
}

#[register_pass(c_abi)]
#[derive(Clone, Copy)]
struct ArrayAbi {
    values: [u64; 2],
}

#[register_pass]
#[derive(Clone, Copy)]
struct Small {
    a: u16,
    b: u16,
    c: u16,
    d: u16,
}

#[register_pass(crate = "::uarch")]
#[derive(Clone, Copy)]
struct Phantom {
    value: u64,
    marker: core::marker::PhantomData<u64>,
}
