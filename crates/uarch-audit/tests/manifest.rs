#![allow(clippy::unwrap_used)]
use uarch_audit::{
    gem5_stats::{Gem5Point, LinearityGate, parse_gem5_stats},
    manifest::{ProbeManifest, ProbeSpec},
};
fn manifest() -> ProbeManifest {
    ProbeManifest {
        probes: vec![ProbeSpec {
            name: "probe".into(),
            manifest_path: "/work/Cargo.toml".into(),
            target_name: "probe".into(),
            hot_symbols: vec!["hot".into()],
            cold_symbols: vec![],
            steps: [10, 30, 70],
            thresholds: LinearityGate::default(),
            allow_integer_division: false,
        }],
    }
}
#[test]
fn table_pins_exact_work_denominators() {
    let table = manifest();
    assert!(table.validate().is_ok());
    let mut points=[10u64,30,70].map(|n| Gem5Point {operations:n,counters:parse_gem5_stats(&format!("system.cpu.numCycles {}\nsystem.cpu.committedInsts {}\nsystem.cpu.committedOps {}\nsystem.cpu.memOrderViolations 0\nsystem.cpu.dcache.overallMshrMisses 0\n",n*4,n*6,n*7)).unwrap()});
    assert_eq!(
        table.verify_gem5("probe", &points).unwrap().cycles_per_op,
        4.0
    );
    points[1].operations = 31;
    assert!(table.verify_gem5("probe", &points).is_err());
    assert!(table.verify_gem5("unknown", &points).is_err());
    let mut duplicate = manifest();
    duplicate.probes.push(duplicate.probes[0].clone());
    assert!(duplicate.validate().is_err());
    let mut invalid = manifest();
    invalid.probes[0].thresholds.min_r_squared = f64::NAN;
    assert!(invalid.validate().is_err());
}
