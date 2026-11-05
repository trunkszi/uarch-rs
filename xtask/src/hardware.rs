use crate::{Error, Result, audit, process};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(serde::Serialize)]
struct VendorProfiler {
    vendor: String,
    tools: Vec<String>,
    status: &'static str,
}

fn vendor_profiler() -> VendorProfiler {
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let vendor = cpuinfo
        .lines()
        .find_map(|line| {
            line.strip_prefix("vendor_id")
                .and_then(|v| v.split_once(':'))
                .map(|(_, v)| v.trim())
        })
        .unwrap_or("unknown");
    let names: &[&str] = match vendor {
        "AuthenticAMD" => &["AMDuProfPcm", "AMDuProfCLI"],
        "GenuineIntel" => &["vtune"],
        _ => &[],
    };
    let mut tools = Vec::new();
    let search = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    for name in names {
        let mut candidates: Vec<_> = search.iter().map(|dir| dir.join(name)).collect();
        if *name == "vtune" {
            candidates.push(PathBuf::from("/opt/intel/oneapi/vtune/latest/bin64/vtune"));
        } else if let Ok(entries) = std::fs::read_dir("/opt") {
            for entry in entries
                .flatten()
                .filter(|entry| entry.file_name().to_string_lossy().starts_with("AMDuProf"))
            {
                candidates.push(entry.path().join("bin").join(name));
            }
        }
        for candidate in candidates {
            use std::os::unix::fs::PermissionsExt;
            if candidate.metadata().is_ok_and(|metadata| {
                metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
            }) {
                tools.push(candidate.display().to_string());
                break;
            }
        }
    }
    // Discovering a profiler is capability evidence, not a completed vendor TMA measurement.
    let status = if tools.is_empty() {
        "Unsupported"
    } else {
        "NotRun"
    };
    VendorProfiler {
        vendor: vendor.to_owned(),
        tools,
        status,
    }
}

pub(crate) fn pmu(executable: &Path, operations: u64, args: &[String]) -> Result<()> {
    println!(
        "vendor-profiler-capability: {}",
        sonic_rs::to_string(&vendor_profiler())?
    );
    if operations == 0 {
        return Err(Error::Gate(
            "Failed: PMU work denominator must be nonzero".into(),
        ));
    }
    let result = Command::new("perf")
        .args([
            "stat",
            "-x",
            ";",
            "-e",
            "cycles:u,instructions:u,branch-misses:u",
            "--",
        ])
        .arg(executable)
        .args(args)
        .output();
    let result = match result {
        Ok(output) => output,
        Err(error) => return Err(Error::Gate(format!("NotRun: perf unavailable: {error}"))),
    };
    let report = String::from_utf8_lossy(&result.stderr);
    if !result.status.success() {
        return Err(Error::Gate(format!(
            "NotRun: PMU measurement failed: {report}"
        )));
    }
    for event in ["cycles:u", "instructions:u", "branch-misses:u"] {
        let rows: Vec<_> = report
            .lines()
            .map(|line| line.split(';').collect::<Vec<_>>())
            .filter(|fields| fields.get(2).is_some_and(|name| name.trim() == event))
            .collect();
        if rows.len() != 1 {
            return Err(Error::Gate(format!("NotRun: missing PMU event {event}")));
        }
        let count: f64 = rows[0][0]
            .trim()
            .parse()
            .map_err(|_| Error::Gate(format!("Unsupported: PMU event {event}")))?;
        if !count.is_finite() || count < 0.0 {
            return Err(Error::Gate(format!("NotRun: invalid PMU event {event}")));
        }
        if rows[0]
            .get(4)
            .and_then(|field| field.trim().parse::<f64>().ok())
            .is_some_and(|percent| percent < 99.0)
        {
            return Err(Error::Gate(format!(
                "NotRun: multiplexed PMU event {event}"
            )));
        }
        println!(
            "{event}: {} per operation ({operations} operations)",
            count / operations as f64
        );
    }
    print!("{}", String::from_utf8_lossy(&result.stdout));
    Ok(())
}

pub(crate) fn miri(root: &Path) -> Result<()> {
    if !root.join("crates/uarch-arch/Cargo.toml").exists() {
        return Err(Error::Gate(
            "NotRun: uarch-arch has not been integrated".into(),
        ));
    }
    let toolchain = std::env::var("UARCH_MIRI_TOOLCHAIN").unwrap_or_else(|_| "nightly".into());
    let toolchain = format!("+{}", toolchain.trim_start_matches('+'));
    for flags in ["", "-Zmiri-tree-borrows"] {
        for package in ["uarch-core", "uarch-arch"] {
            process::run(
                process::cargo(&toolchain, root)
                    .args(["miri", "test", "--locked", "-p", package])
                    .env("MIRIFLAGS", flags),
            )?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum Probe {
    #[value(name = "bitword_low_matches")]
    BitwordLowMatches,
    #[value(name = "bitword_high_matches")]
    BitwordHighMatches,
    #[value(name = "flow_select_register_u64")]
    FlowSelectRegisterU64,
    #[value(name = "flow_select_u64")]
    FlowSelectU64,
    #[value(name = "fast_divisor_u64_div")]
    FastDivisorU64Div,
    #[value(name = "fast_divisor_u32_rem")]
    FastDivisorU32Rem,
    #[value(name = "fast_divisor_u64_rem")]
    FastDivisorU64Rem,
    #[value(name = "uarch_arch_scalar_gather")]
    ScalarGather,
}
const BASELINE_PROBES: [Probe; 7] = [
    Probe::BitwordLowMatches,
    Probe::BitwordHighMatches,
    Probe::FlowSelectRegisterU64,
    Probe::FlowSelectU64,
    Probe::FastDivisorU64Div,
    Probe::FastDivisorU32Rem,
    Probe::FastDivisorU64Rem,
];
const PROBES_PER_MODEL: usize = BASELINE_PROBES.len() + 1;
impl Probe {
    fn name(self) -> &'static str {
        match self {
            Self::BitwordLowMatches => "bitword_low_matches",
            Self::BitwordHighMatches => "bitword_high_matches",
            Self::FlowSelectRegisterU64 => "flow_select_register_u64",
            Self::FlowSelectU64 => "flow_select_u64",
            Self::FastDivisorU64Div => "fast_divisor_u64_div",
            Self::FastDivisorU32Rem => "fast_divisor_u32_rem",
            Self::FastDivisorU64Rem => "fast_divisor_u64_rem",
            Self::ScalarGather => "uarch_arch_scalar_gather",
        }
    }
    fn branchless(self) -> bool {
        matches!(
            self,
            Self::BitwordLowMatches
                | Self::BitwordHighMatches
                | Self::FlowSelectRegisterU64
                | Self::FlowSelectU64
        )
    }
}

#[derive(serde::Deserialize)]
struct ArtifactMessage {
    reason: String,
    executable: Option<PathBuf>,
    target: Option<ArtifactTarget>,
}

#[derive(serde::Deserialize)]
struct ArtifactTarget {
    name: String,
}

fn probe_runner(root: &Path) -> Result<PathBuf> {
    let output = process::output(process::cargo(process::PINNED, root).args([
        "build",
        "--locked",
        "--release",
        "-p",
        "xtask",
        "--bin",
        "xtask",
        "--message-format=json",
    ]))?;
    let mut executable = None;
    for line in output
        .stdout
        .split(|&byte| byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let message: ArtifactMessage = sonic_rs::from_slice(line)?;
        if message.reason == "compiler-artifact"
            && message.target.is_some_and(|target| target.name == "xtask")
            && message.executable.is_some()
        {
            executable = message.executable;
        }
    }
    executable.ok_or_else(|| Error::Gate("missing optimized probe runner".into()))
}

pub(crate) fn pmu_probe(root: &Path, probe: Probe, iterations: u64) -> Result<()> {
    let executable = probe_runner(root)?;
    println!(
        "probe={} iterations={iterations}; counts include runner/startup overhead",
        probe.name()
    );
    pmu(
        &executable,
        iterations,
        &[
            "__pmu-probe".into(),
            "--probe".into(),
            probe.name().into(),
            "--iterations".into(),
            iterations.to_string(),
        ],
    )
}

fn probe_input(index: u64) -> u64 {
    (index & 255) | (((index >> 3) & 7) << 8) | (((index >> 4) & 15) << 60)
}
fn probe_checksum(probe: Probe, iterations: u64) -> u64 {
    use std::hint::black_box;
    let divisor64 =
        uarch_probes::gate::FastDivisorU64::new(std::num::NonZeroU64::MIN.saturating_add(96));
    let divisor32 =
        uarch_probes::gate::FastDivisorU32::new(std::num::NonZeroU32::MIN.saturating_add(96));
    let mut checksum = 0;
    for index in 0..iterations {
        let input = black_box(probe_input(index));
        let value = match probe {
            Probe::BitwordLowMatches => u64::from(uarch_probes::bits::bitword_low_matches(input)),
            Probe::BitwordHighMatches => u64::from(uarch_probes::bits::bitword_high_matches(input)),
            Probe::FlowSelectRegisterU64 => uarch_probes::flow::flow_select_register_u64(
                black_box(input & 1 == 0),
                input,
                black_box(!input),
            ),
            Probe::FlowSelectU64 => uarch_probes::flow::flow_select_u64(
                black_box(input & 1 == 0),
                input,
                black_box(!input),
            ),
            Probe::FastDivisorU64Rem => {
                uarch_probes::fast_divisor_u64_rem(black_box(divisor64), input)
            }
            Probe::FastDivisorU64Div => {
                uarch_probes::arithmetic::fast_divisor_u64_div(black_box(divisor64), input)
            }
            Probe::FastDivisorU32Rem => {
                uarch_probes::arithmetic::fast_divisor_u32_rem(black_box(divisor32), input as u32)
                    .into()
            }
            Probe::ScalarGather => uarch_probes::arch::uarch_arch_scalar_gather(
                input,
                black_box(0x5555_5555_5555_5555),
            ),
        };
        checksum = audit::combine_probe_checksum(checksum, index, black_box(value));
    }
    checksum
}
fn reference_probe_checksum(probe: Probe, iterations: u64) -> u64 {
    let mut checksum = 0;
    for i in 0..iterations {
        let input = probe_input(i);
        let value = match probe {
            Probe::BitwordLowMatches => u64::from(input & 0x703 == 0x301),
            Probe::BitwordHighMatches => {
                u64::from(input & 0xf000_0000_0000_0001 == 0x3000_0000_0000_0001)
            }
            Probe::FlowSelectRegisterU64 | Probe::FlowSelectU64 => {
                if input & 1 == 0 {
                    input
                } else {
                    !input
                }
            }
            Probe::FastDivisorU64Div => input / 97,
            Probe::FastDivisorU64Rem => input % 97,
            Probe::FastDivisorU32Rem => (input as u32 % 97).into(),
            Probe::ScalarGather => (0..32).fold(0, |v, b| v | ((input >> (b * 2)) & 1) << b),
        };
        checksum = audit::combine_probe_checksum(checksum, i, value);
    }
    checksum
}
pub(crate) fn run_probe(probe: Probe, iterations: u64) -> Result<()> {
    let checksum = probe_checksum(probe, iterations);
    if checksum != reference_probe_checksum(probe, iterations) {
        return Err(Error::Gate("PMU probe checksum mismatch".into()));
    }
    println!(
        "probe={} iterations={iterations} checksum={checksum}",
        probe.name()
    );
    Ok(())
}
pub(crate) fn gem5_probe(probe: Probe, iterations: u64) -> Result<()> {
    println!(
        "UARCH_GEM5 probe={} n={iterations} checksum={}",
        probe.name(),
        probe_checksum(probe, iterations)
    );
    Ok(())
}
fn verify_guest(text: &str, probe: Probe, n: u64) -> Result<()> {
    let lines: Vec<_> = text
        .lines()
        .filter(|line| line.starts_with("UARCH_GEM5 "))
        .collect();
    let valid = if let [line] = lines.as_slice() {
        let fields: Vec<_> = line.split_whitespace().collect();
        fields.len() == 4
            && fields[1] == format!("probe={}", probe.name())
            && fields[2]
                .strip_prefix("n=")
                .and_then(|v| v.parse::<u64>().ok())
                == Some(n)
            && fields[3]
                .strip_prefix("checksum=")
                .and_then(|v| v.parse::<u64>().ok())
                == Some(reference_probe_checksum(probe, n))
    } else {
        false
    };
    if !valid {
        return Err(Error::Gate(format!(
            "gem5 guest checksum/output mismatch: {} n={n}",
            probe.name()
        )));
    }
    Ok(())
}
fn executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}
fn gem5_binary() -> Result<Option<PathBuf>> {
    if let Some(path) = std::env::var_os("UARCH_GEM5_BIN") {
        let path = PathBuf::from(path);
        if !executable(&path) {
            return Err(Error::Gate("invalid UARCH_GEM5_BIN executable path".into()));
        }
        return Ok(Some(std::fs::canonicalize(path)?));
    }
    for directory in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        for name in ["gem5.fast", "gem5.opt"] {
            let path = directory.join(name);
            if executable(&path) {
                return Ok(Some(path));
            }
        }
    }
    Ok(None)
}
struct Gem5Directory(PathBuf);
impl Drop for Gem5Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
pub(crate) fn gem5(root: &Path, cpu: Option<&str>) -> Result<()> {
    let models = audit::gem5_cpu_models(cpu)?;
    let limit = std::env::var("UARCH_GEM5_MAX_CYCLES_PER_OP")
        .ok()
        .map(|v| {
            v.parse::<f64>()
                .ok()
                .filter(|n| n.is_finite() && *n > 0.0)
                .ok_or_else(|| Error::Gate("invalid UARCH_GEM5_MAX_CYCLES_PER_OP".into()))
        })
        .transpose()?;
    let runner = crate::codegen::gem5_guest(root)?;
    for probe in BASELINE_PROBES.into_iter().chain([Probe::ScalarGather]) {
        let output = process::text(Command::new(&runner).args([
            "__gem5-probe",
            "--probe",
            probe.name(),
            "--iterations",
            "0000000064",
        ]))?;
        verify_guest(&output, probe, 64)?;
    }
    let Some(binary) = gem5_binary()? else {
        return Err(Error::Gate(format!(
            "NotRun: gem5.opt/gem5.fast unavailable (verified native freestanding guest checksums on {PROBES_PER_MODEL} probes)"
        )));
    };
    let mut serial = 0;
    let directory = loop {
        let path = std::env::temp_dir().join(format!("uarch-gem5-{}-{serial}", std::process::id()));
        match std::fs::create_dir(&path) {
            Ok(()) => break Gem5Directory(path),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => serial += 1,
            Err(e) => return Err(e.into()),
        }
    };
    #[derive(serde::Deserialize)]
    struct Profile {
        slot_width: u64,
    }
    let mut widths = Vec::new();
    for model in &models {
        if model.starts_with("o3-a64") != cfg!(target_arch = "aarch64") {
            return Err(Error::Gate(format!(
                "Unsupported: profile {model} does not match runner ISA"
            )));
        }
        let json = process::output(
            Command::new("python3")
                .arg(root.join("scripts/gem5/o3_sim.py"))
                .args(["--profile", model, "--dump-profile-json"]),
        )?;
        let profile: Profile = sonic_rs::from_slice(&json.stdout)?;
        if profile.slot_width == 0 {
            return Err(Error::Gate("invalid O3 profile slot_width".into()));
        }
        widths.push(profile.slot_width);
    }
    let runs = std::thread::scope(|scope| {
        let mut jobs = Vec::new();
        for model in &models {
            for probe in BASELINE_PROBES.into_iter().chain([Probe::ScalarGather]) {
                for n in [512u64, 2560, 4608] {
                    let out = directory.0.join(format!("{model}-{}-{n}", probe.name()));
                    let binary = &binary;
                    let runner = &runner;
                    jobs.push(scope.spawn(move || -> Result<audit::Gem5Counters> {
                        std::fs::create_dir(&out)?;
                        let output = process::output(
                            Command::new("timeout")
                                .arg("120")
                                .arg(binary)
                                .arg("--outdir")
                                .arg(&out)
                                .arg(root.join("scripts/gem5/o3_sim.py"))
                                .args(["--profile", model, "--binary"])
                                .arg(runner)
                                .args([
                                    "--probe",
                                    probe.name(),
                                    "--iterations",
                                    &format!("{n:010}"),
                                ])
                                .env("GLIBC_TUNABLES", "glibc.pthread.rseq=0"),
                        )?;
                        verify_guest(&String::from_utf8_lossy(&output.stdout), probe, n)?;
                        audit::parse_gem5_stats(&std::fs::read_to_string(out.join("stats.txt"))?)
                    }));
                }
            }
        }
        jobs.into_iter()
            .map(|job| {
                job.join()
                    .map_err(|_| Error::Gate("gem5 simulation worker panicked".into()))
                    .and_then(|r| r)
            })
            .collect::<Result<Vec<_>>>()
    })?;
    for (model_index, model) in models.iter().enumerate() {
        for (probe_index, probe) in BASELINE_PROBES
            .into_iter()
            .chain([Probe::ScalarGather])
            .enumerate()
        {
            let offset = (model_index * PROBES_PER_MODEL + probe_index) * 3;
            let delta = audit::check_gem5_linearity(
                &runs[offset],
                512,
                &runs[offset + 1],
                2560,
                &runs[offset + 2],
                4608,
                0.05,
            )?;
            if !delta.has_structure {
                return Err(Error::Gate("missing O3 structural counters".into()));
            }
            let metrics = audit::compute_gem5_metrics(&delta, 2048, widths[model_index])?;
            let budget = limit.unwrap_or(if matches!(probe, Probe::ScalarGather) {
                256.0
            } else {
                32.0
            });
            if metrics.cycles_per_op > budget
                || metrics.mem_order_violations != 0.0
                || (probe.branchless() && metrics.branch_mispredicts_per_op > 0.005)
            {
                return Err(Error::Gate(format!(
                    "gem5-gate: {model}/{} structural/cycle budget failed: {metrics:?}",
                    probe.name()
                )));
            }
            println!(
                "gem5-gate: Pass profile={model} probe={} slot_width={} max_cycles_per_op={budget} metrics={metrics:?}",
                probe.name(),
                widths[model_index]
            );
        }
        println!(
            "gem5-gate: Unsupported extended BMI/VEX/SIMD probes not admitted for profile={model}; baseline coverage is explicit"
        );
    }
    Ok(())
}
