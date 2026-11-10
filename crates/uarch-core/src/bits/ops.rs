use super::{BitFlag, BitPattern, FlagClear, FlagSet};
use core::{
    marker::PhantomData,
    ops::{BitAnd, BitOr, Not},
};

impl<W: Copy> From<BitFlag<W>> for BitPattern<W> {
    #[inline(always)]
    fn from(flag: BitFlag<W>) -> Self {
        Self::ANY.and_flag(flag)
    }
}
impl<W: Copy> From<FlagClear<W>> for BitPattern<W> {
    #[inline(always)]
    fn from(clear: FlagClear<W>) -> Self {
        Self::from_parts(clear.mask(), 0)
    }
}
impl<W> Not for BitFlag<W> {
    type Output = FlagClear<W>;
    #[inline(always)]
    fn not(self) -> Self::Output {
        FlagClear {
            clear_mask: self.mask(),
            _w: PhantomData,
        }
    }
}
impl<W> Not for FlagSet<W> {
    type Output = FlagClear<W>;
    #[inline(always)]
    fn not(self) -> Self::Output {
        FlagClear {
            clear_mask: self.mask(),
            _w: PhantomData,
        }
    }
}

macro_rules! conjunction {
    ($lhs:ident, $rhs:ident) => {
        impl<W: Copy> BitAnd<$rhs<W>> for $lhs<W> {
            type Output = BitPattern<W>;
            #[inline(always)]
            fn bitand(self, rhs: $rhs<W>) -> Self::Output {
                BitPattern::from(self).and(BitPattern::from(rhs))
            }
        }
    };
}
conjunction!(BitFlag, BitFlag);
conjunction!(BitFlag, FlagClear);
conjunction!(BitFlag, BitPattern);
conjunction!(FlagClear, BitFlag);
conjunction!(FlagClear, FlagClear);
conjunction!(FlagClear, BitPattern);
conjunction!(BitPattern, BitFlag);
conjunction!(BitPattern, FlagClear);
conjunction!(BitPattern, BitPattern);

macro_rules! union {
    ($lhs:ident, $rhs:ident) => {
        impl<W> BitOr<$rhs<W>> for $lhs<W> {
            type Output = FlagSet<W>;
            #[inline(always)]
            fn bitor(self, rhs: $rhs<W>) -> Self::Output {
                FlagSet {
                    mask: self.mask() | rhs.mask(),
                    _w: PhantomData,
                }
            }
        }
    };
}
union!(BitFlag, BitFlag);
union!(BitFlag, FlagSet);
union!(FlagSet, BitFlag);
union!(FlagSet, FlagSet);
