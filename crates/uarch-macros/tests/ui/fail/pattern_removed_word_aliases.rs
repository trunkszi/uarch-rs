uarch::bitword! { struct Word(u64) { [0] READY, [1..=3] MODE: u8 } }
fn main() {
    let word = Word::new(4);
    let _ = Word::from_raw(0);
    let _ = word.apply(Word::MODE.patch(2));
    let _ = word.all(Word::MODE.eq(3));
    let _ = word.any(Word::MODE.eq(3));
    let _ = word.none(Word::MODE.eq(3));
}
