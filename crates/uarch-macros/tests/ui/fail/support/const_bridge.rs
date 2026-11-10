use core::marker::PhantomData;

pub struct Word;
impl Word {
    pub const FIELD: () = ();
}

pub mod __private {
    use super::PhantomData;
    pub struct BitPattern<W>(PhantomData<W>);
    impl<W> BitPattern<W> {
        pub const ANY: Self = Self(PhantomData);
        // A trap distinguishes const expression expansion from runtime calls.
        pub const fn and_eq(self, _: (), _: u64) -> Self {
            panic!("pattern must be evaluated at compile time")
        }
    }
    pub struct BitPatch<W>(PhantomData<W>);
    impl<W> BitPatch<W> {
        pub const EMPTY: Self = Self(PhantomData);
        pub const fn with_field(self, _: (), _: u64) -> Self {
            panic!("patch must be evaluated at compile time")
        }
    }
}
