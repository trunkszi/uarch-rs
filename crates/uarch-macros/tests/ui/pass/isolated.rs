use uarch_macros::isolated;

#[isolated]
struct Cursor {
    value: Option<u64>,
}

#[isolated(full)]
struct Full {
    value: [u64; 16],
}

fn requires_proof<T: uarch::__private::NoPadding>() {}

fn main() {
    requires_proof::<Full>();
    let c = Cursor { value: Some(1) };
    assert_eq!(c.value, Some(1));
    assert_eq!(core::mem::align_of::<Cursor>(), 128);
}
