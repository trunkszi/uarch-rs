use super::{Gem5Counters, Result, diff_gem5_counters, failure};

/// One cumulative run with its exact operation denominator.
#[derive(Debug, Clone)]
pub struct Gem5Point {
    pub operations: u64,
    pub counters: Gem5Counters,
}
/// Limits apply to both adjacent intervals, never to startup-inclusive averages.
#[derive(Debug, Clone, Copy)]
pub struct LinearityGate {
    pub max_relative_slope_drift: f64,
    pub min_r_squared: f64,
    pub max_mshr_misses_per_op: f64,
}
impl Default for LinearityGate {
    fn default() -> Self {
        Self {
            max_relative_slope_drift: 0.05,
            min_r_squared: 0.999,
            max_mshr_misses_per_op: 1.0,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct LinearityReport {
    pub cycles_per_op: f64,
    pub insts_per_op: f64,
    pub cycle_r_squared: f64,
    pub instruction_r_squared: f64,
    pub mshr_misses_per_op: f64,
}
fn fit(x: &[f64; 3], y: &[f64; 3], limit: &LinearityGate) -> Result<(f64, f64)> {
    let a = (y[1] - y[0]) / (x[1] - x[0]);
    let b = (y[2] - y[1]) / (x[2] - x[1]);
    if !a.is_finite()
        || !b.is_finite()
        || a <= 0.0
        || b <= 0.0
        || (b - a).abs() / a > limit.max_relative_slope_drift
    {
        return Err(failure(
            "linearity",
            "non-stationary cycles or instructions",
        ));
    }
    let mx = x.iter().sum::<f64>() / 3.0;
    let my = y.iter().sum::<f64>() / 3.0;
    let xx = x.iter().map(|v| (v - mx) * (v - mx)).sum::<f64>();
    let yy = y.iter().map(|v| (v - my) * (v - my)).sum::<f64>();
    let xy = x
        .iter()
        .zip(y)
        .map(|(a, b)| (a - mx) * (b - my))
        .sum::<f64>();
    let r2 = xy * xy / (xx * yy);
    if !r2.is_finite() || r2 < limit.min_r_squared {
        return Err(failure("linearity", "R squared below threshold"));
    }
    Ok((b, r2.min(1.0)))
}
/// Verifies cycles, instructions, memory ordering and true cache-line fill budgets.
/// # Errors
/// Rejects missing counters, changing schemas, invalid limits, nonlinearity or a failed interval budget.
pub fn verify_three_point_linearity(
    points: &[Gem5Point; 3],
    gate: &LinearityGate,
) -> Result<LinearityReport> {
    let [p0, p1, p2] = points;
    if p0.operations == 0
        || p0.operations >= p1.operations
        || p1.operations >= p2.operations
        || p2.operations > (1u64 << 53)
        || !gate.max_relative_slope_drift.is_finite()
        || !(0.0..=1.0).contains(&gate.max_relative_slope_drift)
        || !gate.min_r_squared.is_finite()
        || !(0.0..=1.0).contains(&gate.min_r_squared)
        || !gate.max_mshr_misses_per_op.is_finite()
        || gate.max_mshr_misses_per_op < 0.0
    {
        return Err(failure("linearity", "invalid operation counts or limits"));
    }
    if points
        .iter()
        .any(|p| !p.counters.has_mshr || !p.counters.has_mem_order)
    {
        return Err(failure(
            "linearity",
            "missing MSHR or memory-order counters",
        ));
    }
    let d1 = diff_gem5_counters(&p1.counters, &p0.counters)?;
    let d2 = diff_gem5_counters(&p2.counters, &p1.counters)?;
    let mut max_mshr: f64 = 0.0;
    for (d, n) in [
        (&d1, p1.operations - p0.operations),
        (&d2, p2.operations - p1.operations),
    ] {
        let mshr = d.dcache_mshr_misses / n as f64;
        if d.mem_order_violations != 0.0 || !mshr.is_finite() || mshr > gate.max_mshr_misses_per_op
        {
            return Err(failure(
                "linearity",
                "memory-order violation or MSHR budget exceeded",
            ));
        }
        max_mshr = max_mshr.max(mshr);
    }
    let x = points.each_ref().map(|p| p.operations as f64);
    let (cycles, cr2) = fit(&x, &points.each_ref().map(|p| p.counters.num_cycles), gate)?;
    let (insts, ir2) = fit(
        &x,
        &points.each_ref().map(|p| p.counters.committed_insts),
        gate,
    )?;
    Ok(LinearityReport {
        cycles_per_op: cycles,
        insts_per_op: insts,
        cycle_r_squared: cr2,
        instruction_r_squared: ir2,
        mshr_misses_per_op: max_mshr,
    })
}
