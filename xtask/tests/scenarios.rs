use xtask::audit::parse_gem5_stats;

#[test]
fn v26_status_aliases_take_priority() -> xtask::Result<()> {
    let source = "system.cpu.numCycles 100\nsystem.cpu.commitStats0.numInsts 20\nsystem.cpu.commitStats0.numOps 30\nsystem.cpu.rename.renamedInsts 35\nsystem.cpu.rename.status::Idle 4\nsystem.cpu.rename.status::Squashing 2\nsystem.cpu.commit.branchMispredicts 3\nsystem.cpu.branchPred.condIncorrect 7\n";
    let c = parse_gem5_stats(source)?;
    assert!(c.has_rename_slots);
    assert_eq!(c.rename_idle_cycles, 4.0);
    assert_eq!(c.rename_squash_cycles, 2.0);
    assert_eq!(c.branch_mispredicts, 3.0);
    for prefix in [
        "system.cpu1",
        "system.switch_cpus",
        "board.processor.cores.core",
    ] {
        assert!(parse_gem5_stats(&format!("{source}{prefix}.numCycles 200\n")).is_err());
    }
    assert!(parse_gem5_stats(&format!("{source}system.cpu.numCycles 100\n")).is_err());
    Ok(())
}

#[test]
fn freestanding_guest_has_no_loader() -> xtask::Result<()> {
    use std::{path::Path, process::Command};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let guest = xtask::codegen::gem5_guest(&root)?;
    let headers = Command::new("readelf").args(["-lW"]).arg(&guest).output()?;
    assert!(headers.status.success());
    let headers = String::from_utf8_lossy(&headers.stdout);
    assert!(!headers.contains("INTERP") && !headers.contains("DYNAMIC"));
    for probe in [
        "bitword_low_matches",
        "bitword_high_matches",
        "flow_select_register_u64",
        "flow_select_u64",
        "fast_divisor_u64_div",
        "fast_divisor_u32_rem",
        "uarch_arch_scalar_gather",
    ] {
        let expected = Command::new(env!("CARGO_BIN_EXE_xtask"))
            .args(["__gem5-probe", "--probe", probe, "--iterations", "64"])
            .output()?;
        assert!(expected.status.success());
        for prefix in [vec![], vec!["__gem5-probe"]] {
            let result = Command::new(&guest)
                .args(prefix)
                .args(["--probe", probe, "--iterations", "0000000064"])
                .output()?;
            assert!(result.status.success(), "{probe}");
            let normalize = |bytes: &[u8]| {
                String::from_utf8_lossy(bytes)
                    .split_whitespace()
                    .map(|field| {
                        if let Some((key, value)) = field.split_once('=')
                            && let Ok(n) = value.parse::<u64>()
                        {
                            format!("{key}={n}")
                        } else {
                            field.to_owned()
                        }
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(normalize(&result.stdout), normalize(&expected.stdout));
        }
    }
    for args in [
        vec!["--probe", "unknown", "--iterations", "1"],
        vec!["--probe", "bitword_low_matches", "--iterations", "0"],
        vec![
            "--probe",
            "bitword_low_matches",
            "--iterations",
            "18446744073709551616",
        ],
        vec!["--probe"],
        vec![
            "--probe",
            "bitword_low_matches",
            "--iterations",
            "1",
            "--iterations",
            "1",
        ],
    ] {
        assert_eq!(Command::new(&guest).args(args).status()?.code(), Some(2));
    }
    Ok(())
}

#[test]
fn absent_simulator_remains_notrun() -> xtask::Result<()> {
    use std::process::Command;
    let paths: Vec<_> = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .filter(|p| !p.join("gem5.opt").exists() && !p.join("gem5.fast").exists())
        .collect();
    let path = std::env::join_paths(paths).map_err(|e| xtask::Error::Gate(e.to_string()))?;
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["gem5-gate", "--cpu", "znver4"])
        .env_remove("UARCH_GEM5_BIN")
        .env_remove("UARCH_GEM5_MAX_CYCLES_PER_OP")
        .env("PATH", path)
        .output()?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("NotRun: gem5.opt/gem5.fast unavailable (verified native freestanding guest checksums on 8 probes)"),"{}",String::from_utf8_lossy(&output.stderr));
    Ok(())
}
