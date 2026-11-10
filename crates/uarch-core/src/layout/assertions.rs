use super::{NoPadding, RegisterLeaf};

/// Requires a padding-free type during constant evaluation.
pub const fn assert_no_padding<T: NoPadding>() {}

/// Requires a scalar ABI leaf during constant evaluation.
pub const fn assert_register_leaf<T: RegisterLeaf>() {}

/// Requires copy semantics during constant evaluation.
pub const fn assert_copy<T: Copy>() {}

/// Lock a type's storage layout and optional field offsets at compile time.
#[macro_export]
macro_rules! assert_layout {
    ($ty:ty, size = $size:expr, align = $align:expr $(, fields = { $($field:ident: $offset:expr),* $(,)? })? $(,)?) => {
        const _: () = {
            ::core::assert!(::core::mem::size_of::<$ty>() == $size);
            ::core::assert!(::core::mem::align_of::<$ty>() == $align);
            $($(::core::assert!(::core::mem::offset_of!($ty, $field) == $offset);)*)?
        };
    };
}
