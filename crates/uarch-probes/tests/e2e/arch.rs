#[cfg(all(target_arch = "x86_64", target_os = "linux", not(miri)))]
mod native {
    use uarch_arch::{Arch, Scalar};

    struct Pinned([u8; 1024]);
    unsafe extern "C" {
        fn sched_getaffinity(pid: i32, size: usize, mask: *mut u8) -> i32;
        fn sched_setaffinity(pid: i32, size: usize, mask: *const u8) -> i32;
    }
    impl Pinned {
        fn current() -> Option<Self> {
            let mut saved = [0u8; 1024];
            // SAFETY: pid 0 names this thread and saved is writable for its full length.
            if unsafe { sched_getaffinity(0, saved.len(), saved.as_mut_ptr()) } != 0 {
                return None;
            }
            let byte = saved.iter().position(|b| *b != 0)?;
            let mut single = [0u8; 1024];
            single[byte] = 1 << saved[byte].trailing_zeros();
            // SAFETY: The mask is readable and selects one CPU from the saved permitted set.
            if unsafe { sched_setaffinity(0, single.len(), single.as_ptr()) } != 0 {
                return None;
            }
            Some(Self(saved))
        }
    }
    impl Drop for Pinned {
        fn drop(&mut self) {
            // SAFETY: The saved affinity mask remains readable for its full length.
            let result = unsafe { sched_setaffinity(0, self.0.len(), self.0.as_ptr()) };
            assert_eq!(result, 0);
        }
    }

    fn supports_v3() -> bool {
        std::is_x86_feature_detected!("avx")
            && std::is_x86_feature_detected!("avx2")
            && std::is_x86_feature_detected!("bmi1")
            && std::is_x86_feature_detected!("bmi2")
            && std::is_x86_feature_detected!("cmpxchg16b")
            && std::is_x86_feature_detected!("f16c")
            && std::is_x86_feature_detected!("fma")
            && std::is_x86_feature_detected!("lzcnt")
            && std::is_x86_feature_detected!("movbe")
            && std::is_x86_feature_detected!("popcnt")
            && std::is_x86_feature_detected!("sse3")
            && std::is_x86_feature_detected!("ssse3")
            && std::is_x86_feature_detected!("sse4.1")
            && std::is_x86_feature_detected!("sse4.2")
            && std::is_x86_feature_detected!("xsave")
    }

    #[test]
    fn tier_probes_match_scalar() {
        let Some(_pin) = Pinned::current() else {
            return;
        };
        if !supports_v3() {
            return;
        }
        let v4 = std::is_x86_feature_detected!("avx512f")
            && std::is_x86_feature_detected!("avx512bw")
            && std::is_x86_feature_detected!("avx512cd")
            && std::is_x86_feature_detected!("avx512dq")
            && std::is_x86_feature_detected!("avx512vl");
        let scalar = Scalar::new();
        let mut state = 0x1234_5678_9abc_def0u64;
        for i in 0..2048 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let mask = if i < 64 {
                1 << i
            } else {
                state.rotate_left(23)
            };
            let keys = [state as i64, i64::MIN, i64::MAX, i];
            let vals = [i64::MAX, 1, i64::MIN, state as i64];
            for target in [i64::MIN, i, state as i64, i64::MAX] {
                let sum = scalar.masked_sum_lt_i64x4(&keys, &vals, target);
                let first = scalar.find_first_ge_i64x4(&keys, target).unwrap_or(4);
                // SAFETY: This thread is pinned to a CPU with complete V3 ISA/OS support;
                // both pointers name initialized four-element arrays for the entire call.
                unsafe {
                    assert_eq!(
                        uarch_probes::arch::uarch_arch_masked_sum(&keys, &vals, target),
                        sum
                    );
                    assert_eq!(
                        uarch_probes::arch::uarch_arch_first_ge(&keys, target),
                        first
                    );
                }
                if v4 {
                    // SAFETY: The pinned CPU also has complete V4 ISA/OS support; arrays are valid.
                    unsafe {
                        assert_eq!(
                            uarch_probes::arch::x86_v4_masked_sum(&keys, &vals, target),
                            sum
                        );
                        assert_eq!(uarch_probes::arch::x86_v4_first_ge(&keys, target), first);
                    }
                }
            }
            let gathered = scalar.bit_gather(state, mask);
            let scattered = scalar.bit_scatter(state, mask);
            assert_eq!(
                uarch_probes::arch::uarch_arch_scalar_gather(state, mask),
                gathered
            );
            // SAFETY: The pinned execution CPU has complete V3 ISA and OS state support.
            unsafe {
                assert_eq!(uarch_probes::arch::x86_v3_gather(state, mask), gathered);
                assert_eq!(uarch_probes::arch::x86_v3_scatter(state, mask), scattered);
                assert_eq!(
                    uarch_probes::arch::x86_v3_slow_pdep_gather(state, mask),
                    gathered
                );
                assert_eq!(
                    uarch_probes::arch::x86_v3_slow_pdep_scatter(state, mask),
                    scattered
                );
            }
            if v4 {
                // SAFETY: The pinned execution CPU has complete V4 ISA and OS state support.
                let actual = unsafe { uarch_probes::arch::x86_v4_gather(state, mask) };
                assert_eq!(actual, gathered);
            }
        }
    }
}
