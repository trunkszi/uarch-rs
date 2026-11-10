#![allow(clippy::unwrap_used)]
use uarch_audit::hardware::{HardwareRun, Profiler, TmaReport, parse_perf_stat};
#[test]
fn perf_counts_require_full_running_time() {
    let text =
        "400;;cycles:u;100;100.00\n600;;instructions:u;100;100.00\n2;;branch-misses:u;100;100.00\n";
    let p = parse_perf_stat(text, 20).unwrap();
    assert_eq!(p.cycles_per_op, 20.0);
    assert_eq!(p.insts_per_op, 30.0);
    assert_eq!(p.branch_misses_per_op, 0.1);
    for bad in [
        text.replace("100.00", "50.00"),
        text.replace("400", "NaN"),
        text.replace("cycles:u", "missing"),
        format!("{text}{text}"),
    ] {
        assert!(parse_perf_stat(&bad, 20).is_err());
    }
    assert!(parse_perf_stat(text, 0).is_err());
}
#[test]
fn every_profiler_uses_one_host_lease() {
    for profiler in [Profiler::Perf, Profiler::AmduProfCli, Profiler::AmduProfPcm] {
        let run = HardwareRun {
            profiler,
            tool: "tool".into(),
            cpu: 3,
            binary: "/bin/probe".into(),
            args: vec!["100".into()],
            output_directory: "/out".into(),
        };
        let cmd = run.command();
        assert_eq!(cmd.get_program(), "flock");
        let args: Vec<_> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert_eq!(&args[..4], ["-w", "180", "/tmp/amduprof.lock", "tool"]);
        assert!(args.windows(3).any(|a| a == ["taskset", "-c", "3"]));
    }
}
#[test]
fn tma_needs_all_real_quadrants() {
    let valid = "Retiring,60\nBad Speculation,10\nFrontend Bound,20\nBackend Bound,10\n";
    let report = TmaReport::parse(valid).unwrap();
    assert!(report.verify([10.0, 20.0, 10.0]).is_ok());
    assert!(report.verify([9.0, 20.0, 10.0]).is_err());
    for bad in [
        valid.replace("Retiring,60", "Retiring,NaN"),
        valid.replace("Retiring,60", "Retiring,10"),
        valid.replace("Frontend Bound,20\n", ""),
        format!("{valid}Retiring,60\n"),
    ] {
        assert!(TmaReport::parse(&bad).is_err());
    }
}
