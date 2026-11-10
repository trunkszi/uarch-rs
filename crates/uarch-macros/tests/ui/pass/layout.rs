use uarch_macros::register_pass;

#[register_pass]
#[cfg(any())]
struct Disabled {
    value: u64,
}

#[uarch_macros::isolated]
#[cfg(any())]
struct DisabledIsolation {
    value: u64,
}

#[uarch_macros::cache_line]
#[cfg(any())]
struct DisabledLine {
    value: [u8; 64],
}

mod nested {
    use super::*;
    #[register_pass(c_abi)]
    #[derive(Clone, Copy)]
    pub struct Inner {
        pub bytes: [u8; 8],
    }
    #[register_pass(c_abi)]
    #[derive(Clone, Copy)]
    pub struct Outer {
        pub inner: Inner,
        pub value: u64,
    }
}

fn require_proof<T: uarch::__private::NoPadding>() {}
fn main() {
    require_proof::<nested::Outer>();
    assert_eq!(core::mem::size_of::<nested::Outer>(), 16);
}
