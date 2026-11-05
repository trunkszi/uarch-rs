//! Each pair preserves results; labels name code patterns, not measured PMU attribution.

pub(super) fn run() {
    let inputs = super::prelude::inputs();
    for (name, bad, good, work_items) in [
        (
            "retiring",
            super::retiring::bad as fn(&[u64; 256]) -> u64,
            super::retiring::good as fn(&[u64; 256]) -> u64,
            256,
        ),
        (
            "bad_speculation",
            super::bad_speculation::bad,
            super::bad_speculation::good,
            super::bad_speculation::WORK_ITEMS,
        ),
        (
            "frontend_bound",
            super::frontend_bound::bad,
            super::frontend_bound::good,
            256,
        ),
        (
            "backend_bound",
            super::backend_bound::bad,
            super::backend_bound::good,
            256,
        ),
    ] {
        assert_eq!(
            bad(&inputs),
            good(&inputs),
            "benchmark results differ: {name}"
        );
        let bad_results = super::private::measure(name, "bad", bad, &inputs, work_items);
        let good_results = super::private::measure(name, "good", good, &inputs, work_items);
        assert_eq!(bad_results, good_results, "sample results differ: {name}");
    }
}
