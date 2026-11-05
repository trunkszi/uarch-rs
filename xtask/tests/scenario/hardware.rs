use std::process::Command;

#[test]
fn named_probe_checks_results() -> xtask::Result<()> {
    for iterations in ["1", "2", "17", "100000"] {
        let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
            .args([
                "__pmu-probe",
                "--probe",
                "flow_select_register_u64",
                "--iterations",
                iterations,
            ])
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout)
                .contains(&format!("iterations={iterations} checksum="))
        );
    }
    for arguments in [
        vec!["pmu-gate", "--probe", "unknown", "--iterations", "1"],
        vec!["pmu-gate", "--probe", "flow_select_register_u64"],
        vec![
            "pmu-gate",
            "--probe",
            "flow_select_register_u64",
            "--iterations",
            "0",
        ],
        vec!["pmu-gate", "--executable", "unused"],
        vec![
            "pmu-gate",
            "--executable",
            "unused",
            "--operations",
            "1",
            "--probe",
            "flow_select_register_u64",
            "--iterations",
            "1",
        ],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
            .args(arguments)
            .output()?;
        assert_eq!(output.status.code(), Some(2));
    }
    Ok(())
}

#[test]
fn gem5_guest_matches_host_oracles() -> xtask::Result<()> {
    for probe in [
        "bitword_low_matches",
        "bitword_high_matches",
        "flow_select_register_u64",
        "flow_select_u64",
        "fast_divisor_u64_div",
        "fast_divisor_u64_rem",
        "fast_divisor_u32_rem",
        "uarch_arch_scalar_gather",
    ] {
        for n in [1u64, 17, 64] {
            let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
                .args([
                    "__gem5-probe",
                    "--probe",
                    probe,
                    "--iterations",
                    &format!("{n:010}"),
                ])
                .output()?;
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let mut checksum = 0;
            for i in 0..n {
                let input = (i & 255) | (((i >> 3) & 7) << 8) | (((i >> 4) & 15) << 60);
                let value = match probe {
                    "bitword_low_matches" => u64::from(input & 0x703 == 0x301),
                    "bitword_high_matches" => {
                        u64::from(input & 0xf000_0000_0000_0001 == 0x3000_0000_0000_0001)
                    }
                    "flow_select_register_u64" | "flow_select_u64" => {
                        if input & 1 == 0 {
                            input
                        } else {
                            !input
                        }
                    }
                    "fast_divisor_u64_div" => input / 97,
                    "fast_divisor_u64_rem" => input % 97,
                    "fast_divisor_u32_rem" => (input as u32 % 97).into(),
                    _ => (0..32).fold(0, |v, b| v | ((input >> (b * 2)) & 1) << b),
                };
                checksum = xtask::audit::combine_probe_checksum(checksum, i, value);
            }
            assert_eq!(
                String::from_utf8_lossy(&output.stdout).trim(),
                format!("UARCH_GEM5 probe={probe} n={n} checksum={checksum}")
            );
        }
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn gem5_process_admission_and_failures() -> xtask::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let directory = std::env::temp_dir().join(format!("uarch-gem5-test-{}", std::process::id()));
    std::fs::create_dir(&directory)?;
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let cleanup = Cleanup(directory);
    let mock = cleanup.0.join("gem5.opt");
    std::fs::write(
        &mock,
        r#"#!/usr/bin/env python3
import os,sys,subprocess
from pathlib import Path
args=sys.argv[1:]
def arg(key): return args[args.index(key)+1]
assert os.environ['GLIBC_TUNABLES']=='glibc.pthread.rseq=0'
ntext=arg('--iterations'); assert len(ntext)==10
n=int(ntext); mode=os.environ.get('MOCK_GEM5_MODE','pass')
result=subprocess.run([arg('--binary'),'__gem5-probe','--probe',arg('--probe'),'--iterations',ntext],capture_output=True,text=True,check=True)
print(result.stdout.strip() if mode!='checksum' else 'UARCH_GEM5 broken checksum')
cycles=100+n*10
if mode=='nonlinear' and n==4608: cycles*=2
branch=0 if mode!='branch' else n
violations=0 if mode!='memory' else n
rename=30+n*5 if mode!='slots' else n*1000
stats=f'''system.cpu.numCycles {cycles}
system.cpu.commitStats0.numInsts {20+n*3}
system.cpu.commitStats0.numOps {30+n*4}
system.cpu.rename.renamedInsts {rename}
system.cpu.rename.idleCycles 0
system.cpu.rename.squashCycles 0
system.cpu.commit.branchMispredicts {branch}
system.cpu.iew.memOrderViolationEvents {violations}
'''
if mode=='nan': stats=stats.replace(str(cycles),'NaN',1)
if mode=='missing': stats=stats.replace('commitStats0.numOps','unknown')
Path(arg('--outdir'),'stats.txt').write_text(stats)
"#,
    )?;
    std::fs::set_permissions(&mock, std::fs::Permissions::from_mode(0o755))?;
    for (mode, budget, message) in [
        ("pass", "24", "gem5-gate: Pass"),
        ("checksum", "24", "checksum/output mismatch"),
        ("nonlinear", "24", "non-stationary O3 ROI"),
        ("branch", "24", "structural/cycle budget failed"),
        ("memory", "24", "structural/cycle budget failed"),
        ("slots", "24", "inconsistent O3 slot accounting"),
        ("nan", "24", "invalid raw counter"),
        ("missing", "24", "missing required"),
        ("pass", "1", "structural/cycle budget failed"),
        ("pass", "NaN", "invalid UARCH_GEM5_MAX_CYCLES_PER_OP"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
            .args(["gem5-gate", "--cpu", "znver4"])
            .env("UARCH_GEM5_BIN", &mock)
            .env("MOCK_GEM5_MODE", mode)
            .env("UARCH_GEM5_MAX_CYCLES_PER_OP", budget)
            .output()?;
        let report = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.status.success(),
            mode == "pass" && budget == "24",
            "{report}"
        );
        assert!(report.contains(message), "{report}");
    }
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["gem5-gate"])
        .env("UARCH_GEM5_BIN", cleanup.0.join("missing"))
        .output()?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid UARCH_GEM5_BIN"));
    Ok(())
}

#[test]
fn model_lists_and_profile_manifest() -> xtask::Result<()> {
    for list in [
        "",
        ",znver4",
        "znver4,",
        "znver4,,znver5",
        "-bad",
        "znver4;touch x",
    ] {
        assert!(xtask::codegen::mca_cpu_models(Some(list)).is_err());
    }
    assert_eq!(
        xtask::codegen::mca_cpu_models(Some("znver4, znver5,znver4"))?,
        vec!["znver4", "znver5"]
    );
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../scripts/gem5/o3_sim.py");
    for (alias, canonical, width) in [
        ("znver4", "o3-w6-rob320", 6),
        ("znver5", "o3-w8-rob448", 8),
        ("sapphirerapids", "o3-w6-rob512", 6),
        ("neoverse-v2", "o3-a64-w6-rob320", 6),
    ] {
        let output = Command::new("python3")
            .arg(&script)
            .args(["--profile", alias, "--dump-profile-json"])
            .output()?;
        assert!(output.status.success());
        let profile: sonic_rs::Value = sonic_rs::from_slice(&output.stdout)?;
        use sonic_rs::JsonValueTrait;
        assert_eq!(
            profile.get("canonical").and_then(|v| v.as_str()),
            Some(canonical)
        );
        assert_eq!(
            profile.get("slot_width").and_then(|v| v.as_u64()),
            Some(width)
        );
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn gem5_profiles_keep_probe_results_separate() -> xtask::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let directory =
        std::env::temp_dir().join(format!("uarch-gem5-profiles-{}", std::process::id()));
    std::fs::create_dir(&directory)?;
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let cleanup = Cleanup(directory);
    let simulator = cleanup.0.join("gem5.opt");
    std::fs::write(
        &simulator,
        r#"#!/usr/bin/env python3
import subprocess,sys
from pathlib import Path
args=sys.argv[1:]
def arg(key): return args[args.index(key)+1]
ntext=arg('--iterations'); n=int(ntext)
probe=arg('--probe'); profile=arg('--profile')
result=subprocess.run([arg('--binary'),'__gem5-probe','--probe',probe,'--iterations',ntext],capture_output=True,text=True,check=True)
print(result.stdout.strip())
scalar=probe=='uarch_arch_scalar_gather'
rate=200 if scalar else {'o3-w6-rob320':10,'o3-w8-rob448':20,'o3-w6-rob512':30}[profile]
Path(arg('--outdir'),'stats.txt').write_text(f'''system.cpu.numCycles {100+n*rate}
system.cpu.commitStats0.numInsts {20+n*3}
system.cpu.commitStats0.numOps {30+n*4}
system.cpu.rename.renamedInsts {30+n*5}
system.cpu.rename.idleCycles 0
system.cpu.rename.squashCycles 0
system.cpu.commit.branchMispredicts {n if scalar else 0}
system.cpu.iew.memOrderViolationEvents 0
''')
"#,
    )?;
    std::fs::set_permissions(&simulator, std::fs::Permissions::from_mode(0o755))?;
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["gem5-gate", "--cpu", "znver4,znver5,sapphirerapids"])
        .env("UARCH_GEM5_BIN", &simulator)
        .env_remove("UARCH_GEM5_MAX_CYCLES_PER_OP")
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(stdout.matches("gem5-gate: Pass profile=").count(), 24);
    for (profile, cycles) in [
        ("o3-w6-rob320", "10.0"),
        ("o3-w8-rob448", "20.0"),
        ("o3-w6-rob512", "30.0"),
    ] {
        let row = stdout
            .lines()
            .find(|line| line.contains(&format!("profile={profile} probe=bitword_low_matches ")))
            .ok_or_else(|| xtask::Error::Gate("missing profile-specific result".into()))?;
        assert!(row.contains(&format!("cycles_per_op: {cycles},")), "{row}");
        assert!(row.contains("branch_mispredicts_per_op: 0.0,"), "{row}");
    }
    Ok(())
}
