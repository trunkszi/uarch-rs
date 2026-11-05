//! Independent lanes so a non-identity `init` is combined once, not once per lane.

/// Non-short-circuit conjunction.
///
/// Every operand is evaluated once. Do not use this to guard a later operand
/// against overflow, a bounds fault, or division by zero.
#[macro_export]
macro_rules! ilp_all {
    ($($cond:expr),+ $(,)?) => {{
        #[allow(clippy::needless_bitwise_bool)]
        let predicate: bool = true $(& ($cond))+;
        predicate
    }};
}

/// Non-short-circuit disjunction.
///
/// Every operand is evaluated once. Do not use this to guard a later operand
/// against overflow, a bounds fault, or division by zero.
#[macro_export]
macro_rules! ilp_any {
    ($($cond:expr),+ $(,)?) => {{
        #[allow(clippy::needless_bitwise_bool)]
        let predicate: bool = false $(| ($cond))+;
        predicate
    }};
}

/// Reduce `xs` on `N` independent lanes.
///
/// `N` is `2`, `4`, or `8`. Each lane starts at `identity`. `init` is combined
/// once with the folded lanes. `combine` must be associative and commutative
/// with that identity. Reassociation changes results that are not a monoid,
/// including floating-point addition.
///
/// # Panics
///
/// Fails at compile time when `N` is not 2, 4, or 8.
#[inline]
pub fn reduce_lanes<const N: usize, T, A: Copy>(
    xs: &[T],
    init: A,
    identity: A,
    step: impl Fn(A, &T) -> A,
    combine: impl Fn(A, A) -> A,
) -> A {
    const {
        assert!(
            N == 2 || N == 4 || N == 8,
            "reduce_lanes lane count must be 2, 4, or 8"
        );
    }
    if N == 2 {
        reduce2(xs, init, identity, &step, &combine)
    } else if N == 4 {
        reduce4(xs, init, identity, &step, &combine)
    } else {
        reduce8(xs, init, identity, &step, &combine)
    }
}

#[inline(always)]
fn reduce2<T, A: Copy>(
    xs: &[T],
    init: A,
    identity: A,
    step: &impl Fn(A, &T) -> A,
    combine: &impl Fn(A, A) -> A,
) -> A {
    let mut a0 = identity;
    let mut a1 = identity;
    let (chunks, rem) = xs.as_chunks::<2>();
    for chunk in chunks {
        a0 = step(a0, &chunk[0]);
        a1 = step(a1, &chunk[1]);
    }
    if let Some(item) = rem.first() {
        a0 = step(a0, item);
    }
    combine(init, combine(a0, a1))
}

#[inline(always)]
fn reduce4<T, A: Copy>(
    xs: &[T],
    init: A,
    identity: A,
    step: &impl Fn(A, &T) -> A,
    combine: &impl Fn(A, A) -> A,
) -> A {
    let mut a0 = identity;
    let mut a1 = identity;
    let mut a2 = identity;
    let mut a3 = identity;
    let (chunks, rem) = xs.as_chunks::<4>();
    for chunk in chunks {
        a0 = step(a0, &chunk[0]);
        a1 = step(a1, &chunk[1]);
        a2 = step(a2, &chunk[2]);
        a3 = step(a3, &chunk[3]);
    }
    if let Some(item) = rem.first() {
        a0 = step(a0, item);
    }
    if let Some(item) = rem.get(1) {
        a1 = step(a1, item);
    }
    if let Some(item) = rem.get(2) {
        a2 = step(a2, item);
    }
    let left = combine(a0, a1);
    let right = combine(a2, a3);
    combine(init, combine(left, right))
}

#[inline(always)]
fn reduce8<T, A: Copy>(
    xs: &[T],
    init: A,
    identity: A,
    step: &impl Fn(A, &T) -> A,
    combine: &impl Fn(A, A) -> A,
) -> A {
    let mut a0 = identity;
    let mut a1 = identity;
    let mut a2 = identity;
    let mut a3 = identity;
    let mut a4 = identity;
    let mut a5 = identity;
    let mut a6 = identity;
    let mut a7 = identity;
    let (chunks, rem) = xs.as_chunks::<8>();
    for chunk in chunks {
        a0 = step(a0, &chunk[0]);
        a1 = step(a1, &chunk[1]);
        a2 = step(a2, &chunk[2]);
        a3 = step(a3, &chunk[3]);
        a4 = step(a4, &chunk[4]);
        a5 = step(a5, &chunk[5]);
        a6 = step(a6, &chunk[6]);
        a7 = step(a7, &chunk[7]);
    }
    if let Some(item) = rem.first() {
        a0 = step(a0, item);
    }
    if let Some(item) = rem.get(1) {
        a1 = step(a1, item);
    }
    if let Some(item) = rem.get(2) {
        a2 = step(a2, item);
    }
    if let Some(item) = rem.get(3) {
        a3 = step(a3, item);
    }
    if let Some(item) = rem.get(4) {
        a4 = step(a4, item);
    }
    if let Some(item) = rem.get(5) {
        a5 = step(a5, item);
    }
    if let Some(item) = rem.get(6) {
        a6 = step(a6, item);
    }
    let p0 = combine(a0, a1);
    let p1 = combine(a2, a3);
    let p2 = combine(a4, a5);
    let p3 = combine(a6, a7);
    combine(init, combine(combine(p0, p1), combine(p2, p3)))
}
