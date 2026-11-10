//! Const-compatible FNV-1a fingerprints for bounded byte sequences.
pub const FNV1A64_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
pub const FNV1A64_PRIME: u64 = 0x0000_0100_0000_01b3;

#[inline(always)]
pub const fn fnv1a64_extend(mut hash: u64, bytes: &[u8]) -> u64 {
    let mut i = 0;
    while i < bytes.len() {
        hash = (hash ^ bytes[i] as u64).wrapping_mul(FNV1A64_PRIME);
        i += 1;
    }
    hash
}
#[inline(always)]
pub const fn fnv1a64(bytes: &[u8]) -> u64 {
    fnv1a64_extend(FNV1A64_OFFSET_BASIS, bytes)
}
