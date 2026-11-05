pub(super) fn inputs() -> [u64; 256] {
    let mut data = [0; 256];
    let mut state = 0x1234_5678_9abc_def0u64;
    for item in &mut data {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *item = state;
    }
    data
}
