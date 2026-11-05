use crate::task_meta::TaskMeta;
use uarch::__private::BitPattern;

#[test]
fn pattern_path_matches_flag() {
    let pat = BitPattern::<TaskMeta>::ANY
        .and_flag(TaskMeta::READY)
        .and_not(TaskMeta::URGENT);
    let ready = TaskMeta::default().patch(uarch::patch!(TaskMeta { READY: true }));
    assert!(ready.matches(pat));
    let urgent = ready.patch(uarch::patch!(TaskMeta { URGENT: true }));
    assert!(!urgent.matches(pat));
}
