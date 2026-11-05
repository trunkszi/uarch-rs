use core::ops::{Deref, DerefMut};

/// Aligns storage to at least 64 bytes; the inner value may require more.
#[repr(C, align(64))]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CacheAligned<T> {
    pub value: T,
}

impl<T> CacheAligned<T> {
    #[inline(always)]
    pub const fn new(value: T) -> Self {
        Self { value }
    }
    #[inline(always)]
    pub const fn get(&self) -> &T {
        &self.value
    }
    #[inline(always)]
    pub const fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.value
    }
}
impl<T> Deref for CacheAligned<T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T {
        self.get()
    }
}
impl<T> DerefMut for CacheAligned<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T {
        self.get_mut()
    }
}
impl<T> AsRef<T> for CacheAligned<T> {
    #[inline(always)]
    fn as_ref(&self) -> &T {
        self.get()
    }
}
impl<T> AsMut<T> for CacheAligned<T> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut T {
        self.get_mut()
    }
}
impl<T> From<T> for CacheAligned<T> {
    #[inline(always)]
    fn from(value: T) -> Self {
        Self::new(value)
    }
}
