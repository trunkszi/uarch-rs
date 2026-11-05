/// Mark the current branch as cold at the call site.
///
/// The hint is inlined so the caller, not this wrapper, is the cold edge.
#[inline(always)]
pub fn cold_path() {
    core::hint::cold_path();
}

#[cold]
#[inline(always)]
const fn cold_path_hint() {
    core::hint::cold_path();
}

/// Identity predicate that marks its false edge as cold. This is a compiler hint.
#[inline(always)]
#[must_use]
pub const fn likely(condition: bool) -> bool {
    if !condition {
        cold_path_hint();
    }
    condition
}

/// Identity predicate that marks its true edge as cold. This is a compiler hint.
#[inline(always)]
#[must_use]
pub const fn unlikely(condition: bool) -> bool {
    if condition {
        cold_path_hint();
    }
    condition
}
