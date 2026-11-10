#![allow(clippy::unwrap_used)]
use uarch_audit::gem5_stats::parse_gem5_stats;
#[test]
fn mshr_is_not_overall_misses() {
    let stats = parse_gem5_stats("system.cpu.numCycles 100\nsystem.cpu.committedInsts 200\nsystem.cpu.committedOps 220\nsystem.cpu.dcache.overallMisses::total 99\nsystem.cpu.dcache.overallMshrMisses::total 7\nsystem.cpu.iew.memOrderViolationEvents 0\n").unwrap();
    assert_eq!(stats.dcache_mshr_misses, 7.0);
    assert!(stats.has_mshr);
    assert_eq!(stats.dcache_misses, 99.0);
}

use uarch_audit::gem5_stats::{Gem5Point, LinearityGate, verify_three_point_linearity};
fn points() -> [Gem5Point; 3] {
    [10u64,30,70].map(|n| Gem5Point { operations:n, counters:parse_gem5_stats(&format!("simTicks {}\nsystem.cpu.numCycles {}\nsystem.cpu.committedInsts {}\nsystem.cpu.committedOps {}\nsystem.cpu.memOrderViolations 2\nsystem.cpu.dcache.overall_mshr_misses {}\n",n*100,100+n*4,20+n*6,30+n*7,5+n)).unwrap() })
}
#[test]
fn differential_gate_cancels_startup() {
    let p = points();
    let gate = LinearityGate::default();
    let r = verify_three_point_linearity(&p, &gate).unwrap();
    assert_eq!(r.cycles_per_op, 4.0);
    assert_eq!(r.insts_per_op, 6.0);
    assert_eq!(r.mshr_misses_per_op, 1.0);
    assert_eq!(r.cycle_r_squared, 1.0);
    assert_eq!(p[0].counters.sim_ticks, Some(1000.0));
}
#[test]
fn each_linearity_dimension_is_enforced() {
    let gate = LinearityGate::default();
    let mut p = points();
    p[2].counters.num_cycles += 400.0;
    assert!(verify_three_point_linearity(&p, &gate).is_err());
    let mut p = points();
    p[2].counters.committed_insts += 300.0;
    assert!(verify_three_point_linearity(&p, &gate).is_err());
    let mut p = points();
    p[1].counters.mem_order_violations += 1.0;
    p[2].counters.mem_order_violations += 1.0;
    assert!(verify_three_point_linearity(&p, &gate).is_err());
    let mut p = points();
    p[1].counters.dcache_mshr_misses += 5.0;
    p[2].counters.dcache_mshr_misses += 5.0;
    assert!(verify_three_point_linearity(&p, &gate).is_err());
    let mut p = points();
    p[0].counters.has_mshr = false;
    assert!(verify_three_point_linearity(&p, &gate).is_err());
    let mut p = points();
    p[1].operations = p[0].operations;
    assert!(verify_three_point_linearity(&p, &gate).is_err());
    let mut p = points();
    p[2].counters.num_cycles += 10.0;
    let strict = LinearityGate {
        max_relative_slope_drift: 1.0,
        min_r_squared: 1.0,
        ..gate
    };
    assert!(verify_three_point_linearity(&p, &strict).is_err());
    for bad in [f64::NAN, f64::INFINITY, -1.0] {
        let mut g = gate;
        g.max_mshr_misses_per_op = bad;
        assert!(verify_three_point_linearity(&points(), &g).is_err());
    }
}
#[test]
fn malformed_stats_fail_closed() {
    let valid =
        "system.cpu.numCycles 100\nsystem.cpu.committedInsts 200\nsystem.cpu.committedOps 220\n";
    for bad in [
        format!("{valid}system.cpu.numCycles 110\n"),
        valid.replace("100", "NaN"),
        valid.replace("committedOps", "unknown"),
        format!("{valid}system.cpu1.numCycles 1\n"),
        format!("{valid}---------- Begin Simulation Statistics ----------\n"),
    ] {
        assert!(parse_gem5_stats(&bad).is_err());
    }
    assert!(parse_gem5_stats("simTicks 100\nsimInsts 20\nsystem.cpu.committedOps 22\n").is_err());
    let fallback = valid.replace("system.cpu.committedInsts", "simInsts");
    assert_eq!(parse_gem5_stats(&fallback).unwrap().committed_insts, 200.0);
}

#[test]
fn startup_jitter_preserves_work_slopes() {
    use uarch_audit::gem5_stats::diff_gem5_counters;
    let mut p = points();
    for (point, n) in p.iter_mut().zip([64, 128, 256]) {
        point.operations = n;
        point.counters.num_cycles = 1000.0 + n as f64 * 4.0;
        point.counters.committed_insts = 20.0 + n as f64 * 6.0;
        point.counters.committed_ops = 30.0 + n as f64 * 7.0;
        point.counters.dcache_mshr_misses = n as f64;
        point.counters.sim_ticks = None;
    }
    for (point, (idle, violations)) in
        p.iter_mut()
            .zip([(2756.0, 111.0), (2682.0, 110.0), (2680.0, 108.0)])
    {
        point.counters.idle_cycles = idle;
        point.counters.mem_order_violations = violations;
    }
    let d = diff_gem5_counters(&p[1].counters, &p[0].counters).unwrap();
    assert_eq!(d.idle_cycles, 0.0);
    assert_eq!(d.mem_order_violations, 0.0);
    let report = verify_three_point_linearity(&p, &LinearityGate::default()).unwrap();
    assert_eq!(report.cycles_per_op, 4.0);
    assert_eq!(report.insts_per_op, 6.0);
    p[2].counters.mem_order_violations = 111.0;
    assert!(verify_three_point_linearity(&p, &LinearityGate::default()).is_err());
    p[2].counters.mem_order_violations = 107.0;
    assert!(verify_three_point_linearity(&p, &LinearityGate::default()).is_err());
}

#[test]
fn work_counters_remain_monotonic() {
    use uarch_audit::gem5_stats::diff_gem5_counters;
    let p = points();
    let low = &p[0].counters;
    for field in 0..6 {
        let mut baseline = low.clone();
        let mut high = p[1].counters.clone();
        match field {
            0 => high.num_cycles = low.num_cycles - 1.0,
            1 => high.committed_insts = low.committed_insts - 1.0,
            2 => high.committed_ops = low.committed_ops - 1.0,
            3 => {
                baseline.renamed_insts = 10.0;
                high.renamed_insts = 9.0;
            }
            4 => {
                baseline.rename_idle_cycles = 10.0;
                high.rename_idle_cycles = 9.0;
            }
            _ => {
                baseline.rename_squash_cycles = 10.0;
                high.rename_squash_cycles = 9.0;
            }
        }
        assert!(diff_gem5_counters(&high, &baseline).is_err());
    }
    for bad in [f64::NAN, f64::INFINITY, -1.0] {
        for idle in [true, false] {
            let mut high = p[1].counters.clone();
            if idle {
                high.idle_cycles = bad;
            } else {
                high.mem_order_violations = bad;
            }
            assert!(diff_gem5_counters(&high, low).is_err());
        }
    }
}
