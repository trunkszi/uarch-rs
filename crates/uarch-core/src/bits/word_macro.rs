/// Defines an unsigned scalar word with disjoint typed flag and field constants.
///
/// ```
/// uarch_core::bitword! {
///     pub struct Status(u64) {
///         [0] pub READY,
///         [8..=11] pub MODE: u8,
///         [32..=63] pub COUNT: u32,
///     }
/// }
/// let mut status = Status::new(0).patch(Status::MODE.patch(3));
/// status |= Status::READY;
/// assert!(status.matches(Status::READY & Status::MODE.eq(3)));
/// ```
///
/// Overlap, backing-width overflow, and narrow value types are rejected:
/// ```compile_fail
/// uarch_core::bitword! { struct Overlap(u64) { [0] FLAG, [0..=3] FIELD: u8 } }
/// ```
/// ```compile_fail
/// uarch_core::bitword! { struct TooWide(u8) { [8] FLAG } }
/// ```
/// ```compile_fail
/// uarch_core::bitword! { struct Narrow(u64) { [0..=15] FIELD: u8 } }
/// ```
/// Read requirements cannot be used to clear a word:
/// ```compile_fail
/// uarch_core::bitword! { struct Status(u64) { [0] READY } }
/// let mut status = Status::default();
/// status &= Status::READY;
/// ```
/// Only fields ending at bit 63 support the topmost-field addition:
/// ```compile_fail
/// uarch_core::bitword! { struct Status(u64) { [8..=11] MODE: u8 } }
/// let _ = Status::default().wrapping_add_top_field(Status::MODE, 1);
/// ```
/// ```compile_fail
/// let field = uarch_core::bits::BitField::<u64, u8, 8, 11>::new();
/// let _ = field.wrapping_add_top(0, 1);
/// ```
#[macro_export]
macro_rules! bitword {
    (
        $(#[$attr:meta])* $vis:vis struct $word:ident($storage:ident) {
            $( [$lo:literal $(..= $hi:literal)?] $field_vis:vis $name:ident $(: $value:ty)? ),* $(,)?
        }
    ) => {
        $(#[$attr])*
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, Default)]
        $vis struct $word(::core::primitive::$storage);

        // SAFETY: A transparent unsigned scalar wrapper has no padding.
        unsafe impl $crate::layout::NoPadding for $word {}
        // SAFETY: The validated backing scalar contributes exactly one ABI leaf.
        unsafe impl $crate::layout::RegisterLeaf for $word { const LEAVES: ::core::primitive::u8 = 1; }

        impl $word {
            $( $crate::bitword!(@descriptor [$lo $(..= $hi)?] $field_vis $name $(: $value)?); )*
            pub const USED_MASK: ::core::primitive::$storage = 0 $(| (Self::$name.mask() as ::core::primitive::$storage))*;
            #[inline(always)]
            pub const fn has_undeclared_bits(self) -> ::core::primitive::bool {
                (self.0 & !Self::USED_MASK) != 0
            }
            #[inline(always)]
            pub const fn try_from_raw(raw: ::core::primitive::$storage) -> ::core::option::Option<Self> {
                if raw & !Self::USED_MASK == 0 { ::core::option::Option::Some(Self(raw)) }
                else { ::core::option::Option::None }
            }
            #[inline(always)]
            pub const fn new(raw: ::core::primitive::$storage) -> Self { Self(raw) }
            #[inline(always)]
            pub const fn test(self, flag: $crate::bits::BitFlag<Self>) -> ::core::primitive::bool {
                flag.test_raw(self.0 as ::core::primitive::u64)
            }
            #[inline(always)]
            pub const fn patch(self, patch: $crate::bits::BitPatch<Self>) -> Self {
                Self(patch.apply(self.0 as ::core::primitive::u64) as ::core::primitive::$storage)
            }
            /// Replaces a field without changing neighboring bits.
            ///
            /// # Errors
            /// Returns `BitError::ValueOverflow` when the value exceeds the field width.
            #[inline(always)]
            pub const fn try_patch<V: $crate::bits::BitValue, const LO: ::core::primitive::u8, const HI: ::core::primitive::u8>(
                self, field: $crate::bits::BitField<Self, V, LO, HI>, raw_value: ::core::primitive::u64,
            ) -> ::core::result::Result<Self, $crate::bits::BitError> {
                if raw_value > field.max_raw() {
                    return ::core::result::Result::Err($crate::bits::BitError::ValueOverflow);
                }
                ::core::result::Result::Ok(Self(((self.0 as ::core::primitive::u64 & !field.mask())
                    | ((raw_value << LO) & field.mask())) as ::core::primitive::$storage))
            }
            /// Replaces a field with a caller-validated raw value.
            ///
            /// # Panics
            /// Panics when the value exceeds the field width.
            #[inline(always)]
            pub const fn patch_raw<V: $crate::bits::BitValue, const LO: ::core::primitive::u8, const HI: ::core::primitive::u8>(
                self, field: $crate::bits::BitField<Self, V, LO, HI>, raw_value: ::core::primitive::u64,
            ) -> Self {
                ::core::assert!(raw_value <= field.max_raw());
                Self(((self.0 as ::core::primitive::u64 & !field.mask())
                    | ((raw_value << LO) & field.mask())) as ::core::primitive::$storage)
            }
            #[inline(always)]
            pub const fn raw(self) -> ::core::primitive::$storage { self.0 }
            #[inline(always)]
            pub const fn matches(self, pattern: $crate::bits::BitPattern<Self>) -> ::core::primitive::bool {
                pattern.matches(self.0 as ::core::primitive::u64)
            }
            #[inline(always)]
            pub const fn get<V: $crate::bits::BitValue, const LO: ::core::primitive::u8, const HI: ::core::primitive::u8>(
                self, field: $crate::bits::BitField<Self, V, LO, HI>,
            ) -> V { field.get(self.0 as ::core::primitive::u64) }
            /// Adds to a field ending at `HI == 63`, the MSB of a 64-bit backing word.
            #[inline(always)]
            pub const fn wrapping_add_top_field<V: $crate::bits::BitValue, const LO: ::core::primitive::u8>(
                self, field: $crate::bits::BitField<Self, V, LO, 63>, delta: ::core::primitive::$storage,
            ) -> Self { Self(field.wrapping_add_top(self.0 as ::core::primitive::u64, delta as ::core::primitive::u64) as ::core::primitive::$storage) }
            #[inline(always)]
            pub const fn wrapping_add_field<V: $crate::bits::BitValue, const LO: ::core::primitive::u8, const HI: ::core::primitive::u8>(
                self, field: $crate::bits::BitField<Self, V, LO, HI>, delta: ::core::primitive::$storage,
            ) -> Self { Self(field.wrapping_add_field(self.0 as ::core::primitive::u64, delta as ::core::primitive::u64) as ::core::primitive::$storage) }
        }
        impl ::core::convert::From<::core::primitive::$storage> for $word {
            #[inline(always)]
            fn from(raw: ::core::primitive::$storage) -> Self { Self::new(raw) }
        }
        impl ::core::convert::From<$word> for ::core::primitive::$storage {
            #[inline(always)]
            fn from(word: $word) -> Self { word.raw() }
        }
        $crate::bitword!(@assign $word $storage BitOrAssign bitor_assign BitFlag |);
        $crate::bitword!(@assign $word $storage BitOrAssign bitor_assign FlagSet |);
        $crate::bitword!(@assign $word $storage BitXorAssign bitxor_assign BitFlag ^);
        $crate::bitword!(@assign $word $storage BitXorAssign bitxor_assign FlagSet ^);
        impl ::core::ops::BitAndAssign<$crate::bits::FlagClear<Self>> for $word {
            #[inline(always)]
            fn bitand_assign(&mut self, clear: $crate::bits::FlagClear<Self>) {
                self.0 &= !(clear.mask() as ::core::primitive::$storage);
            }
        }
        const _: () = {
            $crate::bitword!(@storage $storage);
            $(::core::assert!($lo < <::core::primitive::$storage>::BITS, "flag exceeds backing word");
              $(::core::assert!($hi < <::core::primitive::$storage>::BITS, "field exceeds backing word");)?)*
            let masks: &[::core::primitive::u64] = &[$($word::$name.mask()),*];
            let mut used = 0u64;
            let mut i = 0;
            while i < masks.len() {
                ::core::assert!(used & masks[i] == 0, "overlapping bitword declarations");
                used |= masks[i];
                i += 1;
            }
        };
    };
    (@storage u8) => {};
    (@storage u16) => {};
    (@storage u32) => {};
    (@storage u64) => {};
    (@storage $other:ident) => { ::core::compile_error!("bitword backing type must be u8, u16, u32, or u64"); };
    (@descriptor [$bit:literal] $vis:vis $name:ident) => {
        $vis const $name: $crate::bits::BitFlag<Self> = $crate::bits::BitFlag::new($bit);
    };
    (@descriptor [$lo:literal ..= $hi:literal] $vis:vis $name:ident : $value:ty) => {
        $vis const $name: $crate::bits::BitField<Self, $value, $lo, $hi> = $crate::bits::BitField::new();
    };
    (@assign $word:ident $storage:ident $trait:ident $method:ident $rhs:ident $op:tt) => {
        impl ::core::ops::$trait<$crate::bits::$rhs<Self>> for $word {
            #[inline(always)]
            fn $method(&mut self, rhs: $crate::bits::$rhs<Self>) {
                self.0 = self.0 $op rhs.mask() as ::core::primitive::$storage;
            }
        }
    };
}
