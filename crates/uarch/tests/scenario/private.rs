use crate::task_meta::TaskMeta;
use uarch::prelude::*;

#[test]
fn clear_flag_uses_not() {
    let mut meta = TaskMeta::default();
    meta |= TaskMeta::READY;
    assert!(meta.matches(pattern!(TaskMeta { READY })));
    meta &= !TaskMeta::READY;
    assert!(meta.matches(pattern!(TaskMeta { !READY })));
}
