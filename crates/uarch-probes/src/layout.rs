//! Positive and register-pressure control signatures for ABI audits.

#[uarch_macros::register_pass(crate = "::uarch_core")]
#[derive(Debug, Clone, Copy)]
pub struct RegisterPair {
    pub lo: u64,
    pub hi: u64,
}

const _: () = {
    assert!(core::mem::offset_of!(RegisterPair, lo) == 0);
    assert!(core::mem::offset_of!(RegisterPair, hi) == 8);
    assert!(core::mem::size_of::<RegisterPair>() == 16);
};

#[unsafe(no_mangle)]
#[inline(never)]
pub fn register_pair(pair: RegisterPair) -> RegisterPair {
    RegisterPair {
        lo: pair.lo.wrapping_add(1),
        hi: pair.hi.wrapping_add(2),
    }
}

// The C ABI exposes register exhaustion as an explicit LLVM byval argument.
#[allow(clippy::too_many_arguments)]
#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn register_pressure_control(
    a: u64,
    b: u64,
    c: u64,
    d: u64,
    e: u64,
    f: u64,
    pair: RegisterPair,
) -> RegisterPair {
    let sum = a
        .wrapping_add(b)
        .wrapping_add(c)
        .wrapping_add(d)
        .wrapping_add(e)
        .wrapping_add(f);
    RegisterPair {
        lo: pair.lo.wrapping_add(sum),
        hi: pair.hi.wrapping_add(sum),
    }
}

#[uarch_macros::register_pass(crate = "::uarch_core")]
#[derive(Debug, Clone, Copy)]
pub struct RegisterSmall {
    pub a: u16,
    pub b: u16,
    pub c: u16,
    pub d: u16,
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn register_small(val: RegisterSmall) -> RegisterSmall {
    RegisterSmall {
        a: val.a.wrapping_add(1),
        b: val.b.wrapping_add(2),
        c: val.c.wrapping_add(3),
        d: val.d.wrapping_add(4),
    }
}

#[uarch_macros::register_pass(c_abi, crate = "::uarch_core")]
#[derive(Debug, Clone, Copy)]
pub struct RegisterCAbi {
    pub words: [u32; 3],
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn register_c_abi(val: RegisterCAbi) -> RegisterCAbi {
    RegisterCAbi {
        words: [
            val.words[0].wrapping_add(1),
            val.words[1].wrapping_add(2),
            val.words[2].wrapping_add(3),
        ],
    }
}
