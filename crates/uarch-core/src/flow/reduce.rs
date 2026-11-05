/// Four-lane reduction. `init` and `identity` are each evaluated once.
///
/// `step` is `|acc, item| ...` and `combine` is `|left, right| ...`.
#[macro_export]
macro_rules! reduce_four_way {
    ($xs:expr, $init:expr, $identity:expr, $step:expr, $combine:expr $(,)?) => {
        $crate::flow::reduce_lanes::<4, _, _>($xs, $init, $identity, $step, $combine)
    };
}
