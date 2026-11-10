use core::cell::Cell;
use uarch_core::layout::Isolated;

#[test]
fn isolation_preserves_drop() {
    struct Tracked<'a>(&'a Cell<u32>);
    impl Drop for Tracked<'_> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let drops = Cell::new(0);
    let mut isolated = Isolated::new(Tracked(&drops));
    assert_eq!((&isolated as *const _ as usize) % 128, 0);
    assert!(core::ptr::eq(isolated.get(), &*isolated));
    let pointer = isolated.get_mut() as *mut Tracked<'_>;
    assert!(core::ptr::eq(pointer, &mut *isolated as *mut _));
    let inner = isolated.into_inner();
    assert_eq!(drops.get(), 0);
    drop(inner);
    assert_eq!(drops.get(), 1);
}
