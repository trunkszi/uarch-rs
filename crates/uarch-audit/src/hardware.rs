//! Measurements acquire one host-wide PMU lease; tool discovery is not acceptance.
use crate::{Error, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
fn failure(detail: impl std::fmt::Display) -> Error {
    Error::Gate(format!("[audit.hardware] {detail}"))
}
#[derive(Debug, Clone, Copy)]
pub enum Profiler {
    Perf,
    AmduProfCli,
    AmduProfPcm,
}
/// Explicit pinning and output ownership keep concurrent tools from sharing PMU state.
#[derive(Debug, Clone)]
pub struct HardwareRun {
    pub profiler: Profiler,
    pub tool: PathBuf,
    pub cpu: u32,
    pub binary: PathBuf,
    pub args: Vec<String>,
    pub output_directory: PathBuf,
}
impl HardwareRun {
    /// Constructs a serialized command without invoking a shell.
    pub fn command(&self) -> Command {
        let mut command = Command::new("flock");
        command
            .args(["-w", "180", "/tmp/amduprof.lock"])
            .arg(&self.tool);
        match self.profiler {
            Profiler::Perf => {
                command.args([
                    "stat",
                    "-x",
                    ";",
                    "-e",
                    "cycles:u,instructions:u,branch-misses:u",
                    "--",
                ]);
            }
            Profiler::AmduProfCli => {
                command
                    .args(["collect", "--config", "assess_ext", "-o"])
                    .arg(&self.output_directory)
                    .arg("--");
            }
            Profiler::AmduProfPcm => {
                command
                    .args([
                        "profile",
                        "-m",
                        "ipc,pipeline_util,l1,l2,tlb,dc,cache_miss",
                        "-c",
                    ])
                    .arg(format!("core={}", self.cpu))
                    .arg("-o")
                    .arg(&self.output_directory)
                    .arg("--");
            }
        }
        command
            .args(["taskset", "-c"])
            .arg(self.cpu.to_string())
            .arg(&self.binary)
            .args(&self.args);
        command
    }
    /// Executes a real measurement, rejecting failed, unsupported or unavailable tools.
    /// # Errors
    /// Returns an error for absent tools, failed profiling, or inaccessible output.
    pub fn run(&self) -> Result<Output> {
        std::fs::create_dir_all(&self.output_directory)?;
        crate::process::output(&mut self.command())
    }
}
#[derive(Debug, Clone, Copy)]
pub struct PerfReport {
    pub cycles_per_op: f64,
    pub insts_per_op: f64,
    pub branch_misses_per_op: f64,
}
/// Parses non-multiplexed perf CSV with explicit denominators.
/// # Errors
/// Rejects missing/duplicate/unavailable events, nonfinite counts and low running percentages.
pub fn parse_perf_stat(text: &str, operations: u64) -> Result<PerfReport> {
    if operations == 0 {
        return Err(failure("zero operation denominator"));
    }
    let count = |event: &str| -> Result<f64> {
        let rows: Vec<_> = text
            .lines()
            .map(|l| l.split(';').collect::<Vec<_>>())
            .filter(|f| f.get(2).is_some_and(|v| v.trim() == event))
            .collect();
        if rows.len() != 1 {
            return Err(failure(format!("missing or duplicate perf event {event}")));
        }
        let value = rows[0][0].trim().parse::<f64>().map_err(failure)?;
        let running = rows[0]
            .get(4)
            .ok_or_else(|| failure("missing running percentage"))?
            .trim()
            .trim_end_matches('%')
            .parse::<f64>()
            .map_err(failure)?;
        if !value.is_finite()
            || value < 0.0
            || !running.is_finite()
            || !(99.0..=100.0).contains(&running)
        {
            return Err(failure(format!("invalid/multiplexed perf event {event}")));
        }
        Ok(value / operations as f64)
    };
    let report = PerfReport {
        cycles_per_op: count("cycles:u")?,
        insts_per_op: count("instructions:u")?,
        branch_misses_per_op: count("branch-misses:u")?,
    };
    if report.cycles_per_op <= 0.0 || report.insts_per_op <= 0.0 {
        return Err(failure("no measured work"));
    }
    Ok(report)
}
/// Measures perf under the same global lease used by vendor profilers.
/// # Errors
/// Rejects a failed command or invalid perf report.
pub fn perf_stat(binary: &Path, args: &[String], operations: u64, cpu: u32) -> Result<PerfReport> {
    if operations == 0 {
        return Err(failure("zero operation denominator"));
    }
    let request = HardwareRun {
        profiler: Profiler::Perf,
        tool: "perf".into(),
        cpu,
        binary: binary.to_owned(),
        args: args.to_vec(),
        output_directory: std::env::current_dir()?,
    };
    let result = request.run()?;
    print!("{}", String::from_utf8_lossy(&result.stdout));
    parse_perf_stat(&String::from_utf8_lossy(&result.stderr), operations)
}
#[derive(Debug, Clone, Copy)]
pub struct TmaReport {
    pub retiring: f64,
    pub bad_speculation: f64,
    pub frontend_bound: f64,
    pub backend_bound: f64,
}
impl TmaReport {
    /// Reads vendor-exported name/value percentage rows (CSV or semicolon-delimited).
    /// # Errors
    /// Rejects incomplete, duplicate, unsupported, nonfinite or inconsistent top-down percentages.
    pub fn parse(text: &str) -> Result<Self> {
        let mut values = [None; 4];
        for line in text.lines() {
            let fields: Vec<_> = line
                .split([',', ';'])
                .map(|v| v.trim().trim_matches('"'))
                .collect();
            if fields.len() != 2 {
                continue;
            }
            let i = match fields[0] {
                "Retiring" => 0,
                "Bad Speculation" | "BadSpeculation" | "Bad_Speculation" => 1,
                "Frontend Bound" | "FrontendBound" | "Frontend_Bound" => 2,
                "Backend Bound" | "BackendBound" | "Backend_Bound" => 3,
                _ => continue,
            };
            let v = fields[1]
                .trim_end_matches('%')
                .parse::<f64>()
                .map_err(failure)?;
            if !v.is_finite() || !(0.0..=100.0).contains(&v) || values[i].replace(v).is_some() {
                return Err(failure("invalid/duplicate TMA percentage"));
            }
        }
        let [
            Some(retiring),
            Some(bad_speculation),
            Some(frontend_bound),
            Some(backend_bound),
        ] = values
        else {
            return Err(failure("missing TMA quadrant"));
        };
        if (retiring + bad_speculation + frontend_bound + backend_bound - 100.0).abs() > 1.0 {
            return Err(failure("inconsistent TMA total"));
        }
        Ok(Self {
            retiring,
            bad_speculation,
            frontend_bound,
            backend_bound,
        })
    }
    /// Applies explicit maximum non-retiring percentages.
    /// # Errors
    /// Rejects invalid budgets, corrupted public values, or exceeded quadrants.
    pub fn verify(&self, maximums: [f64; 3]) -> Result<()> {
        let values = [
            self.retiring,
            self.bad_speculation,
            self.frontend_bound,
            self.backend_bound,
        ];
        if values
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=100.0).contains(v))
            || (values.iter().sum::<f64>() - 100.0).abs() > 1.0
            || maximums
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=100.0).contains(v))
            || values[1..].iter().zip(maximums).any(|(v, m)| *v > m)
        {
            return Err(failure("TMA budget failed"));
        }
        Ok(())
    }
}
