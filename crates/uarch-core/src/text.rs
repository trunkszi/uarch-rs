//! Fixed-capacity, zero-padded ASCII storage.

/// ASCII constructors reject or stop at NUL and non-ASCII bytes.
/// The public storage may contain arbitrary bytes; invalid UTF-8 displays as empty.
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixedAscii<const N: usize>(pub [u8; N]);

// SAFETY: The sole byte-array field has alignment one and occupies every byte.
unsafe impl<const N: usize> crate::layout::NoPadding for FixedAscii<N> {}

impl<const N: usize> FixedAscii<N> {
    #[inline(always)]
    pub const fn empty() -> Self {
        Self([0; N])
    }

    #[inline(always)]
    pub const fn try_from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > N {
            return None;
        }
        let mut out = Self::empty();
        let mut i = 0;
        while i < bytes.len() {
            let byte = bytes[i];
            if byte == 0 || byte > 0x7f {
                return None;
            }
            out.0[i] = byte;
            i += 1;
        }
        Some(out)
    }

    #[inline(always)]
    pub const fn try_from_str(s: &str) -> Option<Self> {
        Self::try_from_bytes(s.as_bytes())
    }

    /// Copies at most `N` ASCII bytes, stopping at the first NUL or non-ASCII byte.
    #[inline(always)]
    pub const fn from_bytes_truncating(bytes: &[u8]) -> Self {
        let mut out = Self::empty();
        let mut i = 0;
        while i < bytes.len() && i < N {
            let byte = bytes[i];
            if byte == 0 || byte > 0x7f {
                break;
            }
            out.0[i] = byte;
            i += 1;
        }
        out
    }

    #[inline(always)]
    pub const fn from_str_truncating(s: &str) -> Self {
        Self::from_bytes_truncating(s.as_bytes())
    }

    #[inline(always)]
    pub const fn len(&self) -> usize {
        let mut i = 0;
        while i < N && self.0[i] != 0 {
            i += 1;
        }
        i
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline(always)]
    pub const fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    #[inline(always)]
    pub fn as_trimmed_bytes(&self) -> &[u8] {
        &self.0[..self.len()]
    }

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_trimmed_bytes()).unwrap_or("")
    }

    #[inline(always)]
    pub const fn fnv1a64(&self) -> u64 {
        let mut hash = crate::hash::FNV1A64_OFFSET_BASIS;
        let mut i = 0;
        while i < N && self.0[i] != 0 {
            hash = (hash ^ self.0[i] as u64).wrapping_mul(crate::hash::FNV1A64_PRIME);
            i += 1;
        }
        hash
    }
}

impl<const N: usize> Default for FixedAscii<N> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<const N: usize> core::fmt::Debug for FixedAscii<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("FixedAscii").field(&self.as_str()).finish()
    }
}

impl<const N: usize> core::fmt::Display for FixedAscii<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<const N: usize> AsRef<[u8]> for FixedAscii<N> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<const N: usize> AsRef<str> for FixedAscii<N> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<const N: usize> PartialEq<str> for FixedAscii<N> {
    fn eq(&self, other: &str) -> bool {
        self.as_trimmed_bytes() == other.as_bytes()
    }
}

impl<const N: usize> PartialEq<&str> for FixedAscii<N> {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}
