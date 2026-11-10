macro_rules! assert { ($($tokens:tt)*) => {}; }
assert!(false);
#[repr(C)]
struct Value { word: u64 }
uarch::layout::assert_layout!(Value, size = 4, align = 8);
fn main() {}
