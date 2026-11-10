uarch::bitword! { pub struct Word(u64) { [0] pub READY, [8..=11] pub STATE:u8, } }
fn main() {
    let _ = uarch_macros::pattern!(Word { READY & !READY });
}
