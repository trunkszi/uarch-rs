uarch::bitword! {
    pub struct TaskMeta(u64) {
        [0] pub READY,
        [1] pub ACTIVE,
        [2] pub URGENT,
        [8..=11] pub STATE: u8,
        [12..=15] pub OWNER: u8,
        [32..=63] pub SEQ: u32,
    }
}
