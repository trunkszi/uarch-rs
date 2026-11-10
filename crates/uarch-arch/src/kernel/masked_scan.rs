#[inline(always)]
pub(crate) fn sum(keys: &[i64; 4], vals: &[i64; 4], limit: i64) -> i64 {
    let mut acc = [0i64; 4];
    for i in 0..4 {
        let mask = -i64::from(keys[i] < limit);
        acc[i] = vals[i] & mask;
    }
    let sum01 = acc[0].wrapping_add(acc[1]);
    let sum23 = acc[2].wrapping_add(acc[3]);
    sum01.wrapping_add(sum23)
}

#[inline(always)]
pub(crate) fn find(keys: &[i64; 4], target: i64) -> Option<u32> {
    let mut mask = 0u32;
    for (i, key) in keys.iter().enumerate() {
        mask |= u32::from(*key >= target) << i;
    }
    (mask != 0).then(|| mask.trailing_zeros())
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
pub(crate) unsafe fn sum_avx2(keys: &[i64; 4], vals: &[i64; 4], limit: i64) -> i64 {
    use core::arch::x86_64::*;
    // SAFETY: Each array holds exactly 32 initialized bytes; unaligned loads are permitted.
    let (k, v) = unsafe {
        (
            _mm256_loadu_si256(keys.as_ptr().cast()),
            _mm256_loadu_si256(vals.as_ptr().cast()),
        )
    };
    let keep = _mm256_cmpgt_epi64(_mm256_set1_epi64x(limit), k);
    let selected = _mm256_and_si256(v, keep);
    let lo = _mm256_castsi256_si128(selected);
    let hi = _mm256_extracti128_si256::<1>(selected);
    let pair = _mm_add_epi64(lo, hi);
    let total = _mm_add_epi64(pair, _mm_srli_si128::<8>(pair));
    _mm_cvtsi128_si64(total)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
pub(crate) unsafe fn find_avx2(keys: &[i64; 4], target: i64) -> Option<u32> {
    use core::arch::x86_64::*;
    // SAFETY: The array provides 32 readable bytes; the load allows any alignment.
    let k = unsafe { _mm256_loadu_si256(keys.as_ptr().cast()) };
    let below = _mm256_cmpgt_epi64(_mm256_set1_epi64x(target), k);
    let mask = (!_mm256_movemask_pd(_mm256_castsi256_pd(below)) as u32) & 15;
    (mask != 0).then(|| mask.trailing_zeros())
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
#[inline]
pub(crate) unsafe fn sum_neon(keys: &[i64; 4], vals: &[i64; 4], limit: i64) -> i64 {
    use core::arch::aarch64::*;
    // SAFETY: Two adjacent two-lane loads stay within each four-element array.
    unsafe {
        let bound = vdupq_n_s64(limit);
        let a = vandq_s64(
            vld1q_s64(vals.as_ptr()),
            vreinterpretq_s64_u64(vcltq_s64(vld1q_s64(keys.as_ptr()), bound)),
        );
        let b = vandq_s64(
            vld1q_s64(vals.as_ptr().add(2)),
            vreinterpretq_s64_u64(vcltq_s64(vld1q_s64(keys.as_ptr().add(2)), bound)),
        );
        vaddvq_s64(vaddq_s64(a, b))
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
#[inline]
pub(crate) unsafe fn find_neon(keys: &[i64; 4], target: i64) -> Option<u32> {
    use core::arch::aarch64::*;
    // SAFETY: Two adjacent two-lane loads stay within the four-element array.
    let (a, b) = unsafe {
        let bound = vdupq_n_s64(target);
        (
            vcgeq_s64(vld1q_s64(keys.as_ptr()), bound),
            vcgeq_s64(vld1q_s64(keys.as_ptr().add(2)), bound),
        )
    };
    let mask = (vgetq_lane_u64::<0>(a) & 1)
        | (vgetq_lane_u64::<1>(a) & 2)
        | (vgetq_lane_u64::<0>(b) & 4)
        | (vgetq_lane_u64::<1>(b) & 8);
    (mask != 0).then(|| mask.trailing_zeros())
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f,avx512vl")]
#[inline]
pub(crate) unsafe fn sum_avx512(keys: &[i64; 4], vals: &[i64; 4], limit: i64) -> i64 {
    use core::arch::x86_64::*;
    // SAFETY: The array holds 32 initialized readable bytes; the load permits any alignment.
    let k = unsafe { _mm256_loadu_si256(keys.as_ptr().cast()) };
    let keep = _mm256_cmpgt_epi64_mask(_mm256_set1_epi64x(limit), k);
    // SAFETY: Every enabled lane lies within the four initialized elements of vals.
    let selected = unsafe { _mm256_maskz_loadu_epi64(keep, vals.as_ptr().cast()) };
    let lo = _mm256_castsi256_si128(selected);
    let hi = _mm256_extracti128_si256::<1>(selected);
    let pair = _mm_add_epi64(lo, hi);
    let total = _mm_add_epi64(pair, _mm_srli_si128::<8>(pair));
    _mm_cvtsi128_si64(total)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f,avx512vl")]
#[inline]
pub(crate) unsafe fn find_avx512(keys: &[i64; 4], target: i64) -> Option<u32> {
    use core::arch::x86_64::*;
    // SAFETY: The array holds 32 initialized readable bytes; the load permits any alignment.
    let k = unsafe { _mm256_loadu_si256(keys.as_ptr().cast()) };
    let mask = _mm256_cmpge_epi64_mask(k, _mm256_set1_epi64x(target)) as u32;
    (mask != 0).then(|| mask.trailing_zeros())
}
