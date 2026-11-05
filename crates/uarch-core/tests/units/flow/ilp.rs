use uarch_core::flow::{ilp_all, ilp_any};

#[test]
fn ilp_all_runs_all() {
    let mut hits = 0u32;
    let predicate = ilp_all!(
        {
            hits += 1;
            false
        },
        {
            hits += 1;
            true
        }
    );
    assert!(!predicate);
    assert_eq!(hits, 2);
}

#[test]
fn ilp_any_runs_all() {
    let mut hits = 0u32;
    let predicate = ilp_any!(
        {
            hits += 1;
            true
        },
        {
            hits += 1;
            false
        }
    );
    assert!(predicate);
    assert_eq!(hits, 2);
}
