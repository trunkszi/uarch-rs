/// Failure to build a [`super::Pool`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PoolError {
    /// `storage.len()` was not `1 << LOG2`.
    #[error("[uarch.mem] storage length {len} does not equal capacity {cap}")]
    Capacity { len: usize, cap: usize },
}
