mod arithmetic;
mod bits;
mod flow;
mod layout;
mod mem;
fn main() {
    layout::run();
    bits::run();
    flow::run();
    arithmetic::run();
    mem::run();
}
