//! Additive O3 counters and model attribution; these are not hardware PMU measurements.
type Result<T> = core::result::Result<T, String>;
fn failure(subject: &str, detail: &str) -> String {
    format!("[audit.gem5] {subject}: {detail}")
}
mod linearity;
pub use linearity::{Gem5Point, LinearityGate, LinearityReport, verify_three_point_linearity};
/// Raw additive O3 counters; availability preserves missing attribution inputs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Gem5Counters {
    pub num_cycles: f64,
    pub idle_cycles: f64,
    pub committed_insts: f64,
    pub committed_ops: f64,
    pub renamed_insts: f64,
    pub rename_idle_cycles: f64,
    pub rename_squash_cycles: f64,
    pub commit_squashed_insts: f64,
    pub branch_mispredicts: f64,
    pub rob_full_events: f64,
    pub iq_full_events: f64,
    pub lq_full_events: f64,
    pub sq_full_events: f64,
    pub icache_stall_cycles: f64,
    pub dcache_misses: f64,
    pub dcache_mshr_misses: f64,
    pub has_mshr: bool,
    pub has_mem_order: bool,
    pub sim_ticks: Option<f64>,
    pub mem_order_violations: f64,
    pub cpu_prefix: String,
    pub has_rename_slots: bool,
    pub has_structure: bool,
}
impl Gem5Counters {
    fn values(&self) -> [f64; 17] {
        [
            self.num_cycles,
            self.idle_cycles,
            self.committed_insts,
            self.committed_ops,
            self.renamed_insts,
            self.rename_idle_cycles,
            self.rename_squash_cycles,
            self.commit_squashed_insts,
            self.branch_mispredicts,
            self.rob_full_events,
            self.iq_full_events,
            self.lq_full_events,
            self.sq_full_events,
            self.icache_stall_cycles,
            self.dcache_misses,
            self.mem_order_violations,
            self.dcache_mshr_misses,
        ]
    }
    fn with_values(&self, v: &[f64; 17]) -> Self {
        Self {
            num_cycles: v[0],
            idle_cycles: v[1],
            committed_insts: v[2],
            committed_ops: v[3],
            renamed_insts: v[4],
            rename_idle_cycles: v[5],
            rename_squash_cycles: v[6],
            commit_squashed_insts: v[7],
            branch_mispredicts: v[8],
            rob_full_events: v[9],
            iq_full_events: v[10],
            lq_full_events: v[11],
            sq_full_events: v[12],
            icache_stall_cycles: v[13],
            dcache_misses: v[14],
            mem_order_violations: v[15],
            dcache_mshr_misses: v[16],
            ..self.clone()
        }
    }
}

/// Derived model metrics, explicitly distinct from hardware TMA.
#[derive(Debug)]
pub struct Gem5Metrics {
    pub cycles_per_op: f64,
    pub insts_per_op: f64,
    pub uops_per_op: f64,
    pub ipc: f64,
    pub branch_mispredicts_per_op: f64,
    pub dcache_misses_per_op: f64,
    pub mem_order_violations: f64,
    pub rob_full_per_op: f64,
    pub iq_full_per_op: f64,
    pub lq_full_per_op: f64,
    pub sq_full_per_op: f64,
    pub retiring_pct: f64,
    pub bad_speculation_pct: f64,
    pub frontend_bound_pct: f64,
    pub backend_bound_pct: f64,
    pub dominant_bound: &'static str,
    pub attribution_mode: &'static str,
}

/// Resolves canonical model profiles and inspired-by aliases.
///
/// # Errors
/// Rejects empty elements, unknown profiles and malformed lists.
pub fn gem5_cpu_models(override_list: Option<&str>) -> Result<Vec<String>> {
    let default = if cfg!(target_arch = "aarch64") {
        "o3-a64-w6-rob320"
    } else {
        "o3-w6-rob320,o3-w8-rob448,o3-w6-rob512"
    };
    let mut models = Vec::new();
    for item in override_list.unwrap_or(default).split(',') {
        let model = match item.trim() {
            "o3-w6-rob320" | "zen4-o3" | "znver4" => "o3-w6-rob320",
            "o3-w8-rob448" | "zen5-o3" | "znver5" => "o3-w8-rob448",
            "o3-w6-rob512" | "sapphirerapids-o3" | "sapphirerapids" => "o3-w6-rob512",
            "o3-a64-w6-rob320" | "neoverse-v2-o3" | "neoverse-v2" => "o3-a64-w6-rob320",
            _ => return Err(failure("gem5", "empty or unknown CPU profile")),
        };
        if !models.iter().any(|entry| entry == model) {
            models.push(model.to_owned());
        }
    }
    Ok(models)
}

const GEM5_FIELDS: [&[&str]; 17] = [
    &["numCycles"],
    &["idleCycles"],
    &[
        "commitStats0.numInsts",
        "commit.instsCommitted::total",
        "commit.instsCommitted",
        "committedInsts",
    ],
    &[
        "commitStats0.numOps",
        "commit.opsCommitted::total",
        "commit.opsCommitted",
        "committedOps",
    ],
    &["rename.renamedInsts", "rename.RenamedInsts"],
    &[
        "rename.idleCycles",
        "rename.IdleCycles",
        "rename.status::Idle",
    ],
    &[
        "rename.squashCycles",
        "rename.SquashCycles",
        "rename.status::Squashing",
    ],
    &["commit.commitSquashedInsts", "commit.squashedInsts"],
    &["commit.branchMispredicts", "branchPred.condIncorrect"],
    &["rename.ROBFullEvents"],
    &["rename.IQFullEvents"],
    &["rename.LQFullEvents"],
    &["rename.SQFullEvents"],
    &["fetchStats0.icacheStallCycles", "fetch.icacheStallCycles"],
    &[
        "dcache.overallMisses::total",
        "dcache.overall_misses::total",
    ],
    &["iew.memOrderViolationEvents", "memOrderViolations"],
    &[
        "dcache.overallMshrMisses::total",
        "dcache.overall_mshr_misses::total",
        "dcache.overallMshrMisses",
        "dcache.overall_mshr_misses",
    ],
];

/// Parses a single O3 statistics block, preserving schema and CPU identity.
///
/// # Errors
/// Rejects ambiguous blocks/prefixes, duplicate keys and invalid/missing counters.
pub fn parse_gem5_stats(text: &str) -> Result<Gem5Counters> {
    if text.matches("Begin Simulation Statistics").count() > 1
        || text.matches("Begin Simulation Statistics").count()
            != text.matches("End Simulation Statistics").count()
    {
        return Err(failure("gem5", "ambiguous or truncated statistics blocks"));
    }
    let mut sim_ticks = None;
    let mut sim_insts = None;
    let mut prefix = None;
    let mut stats = std::collections::BTreeMap::new();
    for line in text.lines() {
        let mut words = line.split_whitespace();
        let Some(key) = words.next() else { continue };
        if let Some(cpu) = key.strip_suffix(".numCycles")
            && ![
                "system.cpu",
                "board.processor.cores.core",
                "board.processor.cores0.core",
            ]
            .contains(&cpu)
        {
            return Err(failure("gem5", "unsupported or conflicting CPU prefix"));
        }
        if key == "simTicks" || key == "simInsts" {
            let value = words
                .next()
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|v| v.is_finite() && *v >= 0.0)
                .ok_or_else(|| failure(key, "invalid counter"))?;
            let slot = if key == "simTicks" {
                &mut sim_ticks
            } else {
                &mut sim_insts
            };
            if slot.replace(value).is_some() {
                return Err(failure(key, "duplicate counter"));
            }
            continue;
        }
        let Some((cpu, suffix)) = [
            "system.cpu",
            "board.processor.cores.core",
            "board.processor.cores0.core",
        ]
        .iter()
        .find_map(|cpu| {
            key.strip_prefix(&format!("{cpu}."))
                .map(|suffix| (*cpu, suffix))
        }) else {
            continue;
        };
        if !GEM5_FIELDS.iter().any(|names| names.contains(&suffix)) {
            continue;
        }
        if prefix.is_some_and(|old| old != cpu) {
            return Err(failure("gem5", "conflicting CPU prefixes"));
        }
        prefix = Some(cpu);
        let value = words
            .next()
            .and_then(|word| word.parse::<f64>().ok())
            .filter(|value| value.is_finite() && *value >= 0.0)
            .ok_or_else(|| failure(key, "invalid raw counter"))?;
        if stats.insert(suffix, value).is_some() {
            return Err(failure(key, "duplicate counter"));
        }
    }
    let mut values = [0.0; 17];
    let mut present = [false; 17];
    for (index, names) in GEM5_FIELDS.iter().enumerate() {
        if let Some(value) = names.iter().find_map(|name| stats.get(name)) {
            values[index] = *value;
            present[index] = true;
        }
    }
    if !present[2]
        && let Some(value) = sim_insts
    {
        values[2] = value;
        present[2] = true;
    }
    // Ticks have different units; never label them cycles without a clock conversion.
    if [0, 2, 3].iter().any(|&i| !present[i]) {
        return Err(failure("gem5", "missing required cycles/insts/ops counter"));
    }
    let base = Gem5Counters {
        cpu_prefix: prefix.unwrap_or("").into(),
        has_rename_slots: present[4] && present[5] && present[6],
        has_structure: present[8] && present[15],
        has_mem_order: present[15],
        has_mshr: present[16],
        sim_ticks,
        ..Gem5Counters::default()
    };
    Ok(base.with_values(&values))
}

/// Subtracts raw additive counters without subtracting rates or percentages.
///
/// # Errors
/// Idle time saturates at zero; memory-order setup jitter may fall by at most two.
/// Rejects incompatible identities/availability, invalid counters and decreasing work counters.
pub fn diff_gem5_counters(high: &Gem5Counters, low: &Gem5Counters) -> Result<Gem5Counters> {
    if high.cpu_prefix != low.cpu_prefix
        || high.has_rename_slots != low.has_rename_slots
        || high.has_structure != low.has_structure
        || high.has_mshr != low.has_mshr
        || high.has_mem_order != low.has_mem_order
        || high.sim_ticks.is_some() != low.sim_ticks.is_some()
    {
        return Err(failure("gem5", "incompatible counter snapshots"));
    }
    const IDLE_CYCLES: usize = 1;
    const MEM_ORDER_VIOLATIONS: usize = 15;
    // Why: Bounded setup noise may fall by two events; positive work violations stay visible.
    const SETUP_MEM_ORDER_JITTER: core::ops::RangeInclusive<f64> = -2.0..=0.0;
    let mut values = [0.0; 17];
    for (i, (hi, lo)) in high.values().into_iter().zip(low.values()).enumerate() {
        let delta = hi - lo;
        // Startup/teardown idle time does not grow with the work horizon.
        // Memory-order setup noise is bounded; a positive work slope remains visible.
        values[i] = match i {
            IDLE_CYCLES => delta.max(0.0),
            MEM_ORDER_VIOLATIONS if SETUP_MEM_ORDER_JITTER.contains(&delta) => 0.0,
            _ => delta,
        };
        if !hi.is_finite()
            || !lo.is_finite()
            || hi < 0.0
            || lo < 0.0
            || values[i] < 0.0
            || !values[i].is_finite()
        {
            return Err(failure(
                GEM5_FIELDS[i][0],
                &format!("negative or invalid counter delta: high={hi} low={lo}"),
            ));
        }
    }
    if [0, 2, 3].iter().any(|&i| values[i] <= 0.0) {
        return Err(failure("gem5", "non-positive cycle/inst/op delta"));
    }
    let mut delta = high.with_values(&values);
    delta.sim_ticks = match (high.sim_ticks, low.sim_ticks) {
        (Some(hi), Some(lo)) if hi.is_finite() && lo.is_finite() && hi >= lo => Some(hi - lo),
        (None, None) => None,
        _ => return Err(failure("simTicks", "invalid counter delta")),
    };
    Ok(delta)
}

/// Validates three-run cycle stationarity and returns the second raw interval.
///
/// # Errors
/// Rejects unordered runs, invalid tolerances and drift beyond the supplied bound.
#[expect(
    clippy::too_many_arguments,
    reason = "The gate contract pairs three snapshots with counts and a tolerance."
)]
pub fn check_gem5_linearity(
    r0: &Gem5Counters,
    n0: u64,
    r1: &Gem5Counters,
    n1: u64,
    r2: &Gem5Counters,
    n2: u64,
    max_rel_drift: f64,
) -> Result<Gem5Counters> {
    if n0 >= n1 || n1 >= n2 || !max_rel_drift.is_finite() || !(0.0..=1.0).contains(&max_rel_drift) {
        return Err(failure("gem5", "invalid linearity arguments"));
    }
    let d1 = diff_gem5_counters(r1, r0)?;
    let d2 = diff_gem5_counters(r2, r1)?;
    let first = d1.num_cycles / (n1 - n0) as f64;
    let second = d2.num_cycles / (n2 - n1) as f64;
    if !first.is_finite()
        || !second.is_finite()
        || first <= 0.0
        || second <= 0.0
        || (second - first).abs() / first > max_rel_drift
    {
        return Err(failure("gem5", "non-stationary O3 ROI"));
    }
    Ok(d2)
}

/// Computes the specified model-specific rename-slot attribution, not hardware TMA.
///
/// # Errors
/// Rejects missing attribution inputs, invalid denominators or inconsistent slots.
pub fn compute_gem5_metrics(
    d: &Gem5Counters,
    operations: u64,
    slot_width: u64,
) -> Result<Gem5Metrics> {
    if operations == 0
        || slot_width == 0
        || !d.has_rename_slots
        || d.values().iter().any(|v| !v.is_finite() || *v < 0.0)
        || d.num_cycles <= 0.0
        || d.idle_cycles > d.num_cycles
    {
        return Err(failure("gem5", "invalid or missing O3 metric inputs"));
    }
    let n = operations as f64;
    let slots = (d.num_cycles - d.idle_cycles).max(1.0) * slot_width as f64;
    let retiring = 100.0 * d.committed_ops / slots;
    let bad = 100.0
        * ((d.renamed_insts - d.committed_ops).max(0.0)
            + slot_width as f64 * d.rename_squash_cycles)
        / slots;
    let frontend = 100.0 * slot_width as f64 * d.rename_idle_cycles / slots;
    let sum = retiring + bad + frontend;
    if !sum.is_finite() || sum > 105.0 {
        return Err(failure("gem5", "inconsistent O3 slot accounting"));
    }
    let backend = (100.0 - sum).clamp(0.0, 100.0);
    let buckets = [retiring, bad, frontend, backend];
    let names = [
        "Retiring",
        "Bad Speculation",
        "Frontend Bound",
        "Backend Bound",
    ];
    let mut dominant = 0;
    for i in 1..4 {
        if buckets[i] > buckets[dominant] {
            dominant = i;
        }
    }
    Ok(Gem5Metrics {
        cycles_per_op: d.num_cycles / n,
        insts_per_op: d.committed_insts / n,
        uops_per_op: d.committed_ops / n,
        ipc: d.committed_insts / d.num_cycles,
        branch_mispredicts_per_op: d.branch_mispredicts / n,
        dcache_misses_per_op: d.dcache_misses / n,
        mem_order_violations: d.mem_order_violations,
        rob_full_per_op: d.rob_full_events / n,
        iq_full_per_op: d.iq_full_events / n,
        lq_full_per_op: d.lq_full_events / n,
        sq_full_per_op: d.sq_full_events / n,
        retiring_pct: retiring,
        bad_speculation_pct: bad,
        frontend_bound_pct: frontend,
        backend_bound_pct: backend,
        dominant_bound: names[dominant],
        attribution_mode: "o3-rename-slot",
    })
}
