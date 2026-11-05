use uarch_macros::register_pass;
#[register_pass]
#[derive(Clone, Copy)]
struct Bad {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
}
fn main() {}
