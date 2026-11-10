use std::process::Command;

#[test]
fn help_exposes_all_gates() -> xtask::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("--help")
        .output()?;
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    for gate in [
        "lint",
        "msrv",
        "ui",
        "abi-check",
        "ir-check",
        "asm-check",
        "size-report",
        "verify",
        "mca-gate",
        "pmu-gate",
        "miri",
        "bench",
    ] {
        assert!(help.contains(gate));
    }
    assert!(
        !Command::new(env!("CARGO_BIN_EXE_xtask"))
            .arg("unknown")
            .output()?
            .status
            .success()
    );
    Ok(())
}

#[test]
fn remainder_runner_checks_result() -> xtask::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "__pmu-probe",
            "--probe",
            "fast_divisor_u64_rem",
            "--iterations",
            "1000",
        ])
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("probe=fast_divisor_u64_rem iterations=1000 checksum=")
    );
    Ok(())
}
