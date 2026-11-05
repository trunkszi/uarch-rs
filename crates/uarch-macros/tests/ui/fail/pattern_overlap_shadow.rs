macro_rules! assert { ($($tokens:tt)*) => {}; }
assert!(false);
uarch::bitword! { struct Value(u64) { [0] FIRST, [0] SECOND } }
fn main() {}
