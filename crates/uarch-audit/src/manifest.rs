//! Declarative probe tables bind target identity, exact symbols and measurement budgets.
use crate::{
    Error, Result,
    artifacts::{self, CargoArtifact, EmittedArtifacts},
    gem5_stats::{Gem5Point, LinearityGate, LinearityReport, verify_three_point_linearity},
};
use std::path::{Path, PathBuf};
use std::process::Command;
#[derive(Debug, Clone)]
pub struct ProbeSpec {
    pub name: String,
    pub manifest_path: PathBuf,
    pub target_name: String,
    pub hot_symbols: Vec<String>,
    pub cold_symbols: Vec<String>,
    pub steps: [u64; 3],
    pub thresholds: LinearityGate,
    pub allow_integer_division: bool,
}
#[derive(Debug, Clone)]
pub struct ProbeManifest {
    pub probes: Vec<ProbeSpec>,
}
#[derive(Debug, Clone)]
pub struct LlvmTools {
    pub nm: PathBuf,
    pub objdump: PathBuf,
    pub readobj: PathBuf,
    pub size: PathBuf,
}
#[derive(Debug)]
pub struct ArtifactReport {
    pub probe: String,
    pub cargo: CargoArtifact,
    pub emitted: EmittedArtifacts,
    pub symbols: String,
    pub sections: String,
    pub sizes: String,
}
fn failure(detail: impl std::fmt::Display) -> Error {
    Error::Gate(format!("[audit.manifest] {detail}"))
}
impl ProbeManifest {
    /// Checks table uniqueness, nonempty symbol coverage, step ordering and finite limits.
    /// # Errors
    /// Rejects malformed declarations before tools are executed.
    pub fn validate(&self) -> Result<()> {
        let mut names = std::collections::BTreeSet::new();
        if self.probes.is_empty() {
            return Err(failure("empty probe table"));
        }
        for p in &self.probes {
            if p.name.is_empty()
                || !names.insert(&p.name)
                || p.target_name.is_empty()
                || p.hot_symbols.is_empty()
                || p.hot_symbols
                    .iter()
                    .chain(&p.cold_symbols)
                    .any(|s| s.is_empty())
                || p.steps[0] == 0
                || p.steps[0] >= p.steps[1]
                || p.steps[1] >= p.steps[2]
                || p.steps[2] > (1 << 53)
            {
                return Err(failure("invalid/duplicate probe or unordered steps"));
            }
            let t = p.thresholds;
            if !t.max_relative_slope_drift.is_finite()
                || !(0.0..=1.0).contains(&t.max_relative_slope_drift)
                || !t.min_r_squared.is_finite()
                || !(0.0..=1.0).contains(&t.min_r_squared)
                || !t.max_mshr_misses_per_op.is_finite()
                || t.max_mshr_misses_per_op < 0.0
            {
                return Err(failure("invalid thresholds"));
            }
        }
        Ok(())
    }
    /// Audits every table entry against a single Cargo JSON stream and caller-selected LLVM tools.
    /// # Errors
    /// Rejects invalid tables, missing emissions, failed tools, bad placement or hot-symbol rules.
    pub fn audit_artifacts(
        &self,
        cargo_json: &str,
        tools: &LlvmTools,
    ) -> Result<Vec<ArtifactReport>> {
        self.validate()?;
        let mut reports = Vec::new();
        for p in &self.probes {
            let cargo = artifacts::select_artifact(cargo_json, &p.manifest_path, &p.target_name)?;
            let emitted = cargo.emitted()?;
            let text = std::fs::read_to_string(&emitted.ir)?;
            let dump = crate::process::text(
                Command::new(&tools.objdump)
                    .args(["-d", "--no-show-raw-insn"])
                    .arg(&emitted.object),
            )?;
            let table =
                crate::process::text(Command::new(&tools.objdump).arg("-t").arg(&emitted.object))?;
            artifacts::verify_cold_sections(&table, &p.cold_symbols)?;
            for s in &p.hot_symbols {
                artifacts::verify_hot_symbol(&dump, &text, s, p.allow_integer_division)?;
            }
            let symbols = crate::process::text(
                Command::new(&tools.nm)
                    .args(["--defined-only", "--format=posix"])
                    .arg(&emitted.object),
            )?;
            let sections = crate::process::text(
                Command::new(&tools.readobj)
                    .args(["--sections", "--unwind"])
                    .arg(&emitted.object),
            )?;
            let sizes = crate::process::text(Command::new(&tools.size).arg(&emitted.object))?;
            reports.push(ArtifactReport {
                probe: p.name.clone(),
                cargo,
                emitted,
                symbols,
                sections,
                sizes,
            });
        }
        Ok(reports)
    }
    /// Matches measured denominators to the registered probe and applies all O3 limits.
    /// # Errors
    /// Rejects an unknown probe, changed step counts or failed differential gates.
    pub fn verify_gem5(&self, name: &str, points: &[Gem5Point; 3]) -> Result<LinearityReport> {
        self.validate()?;
        let probe = self
            .probes
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| failure("unregistered probe"))?;
        if points.each_ref().map(|p| p.operations) != probe.steps {
            return Err(failure("measurement steps differ from declaration"));
        }
        verify_three_point_linearity(points, &probe.thresholds).map_err(Error::Gate)
    }
}
/// Exact executable identity can be persisted beside both PMU and gem5 results.
/// # Errors
/// Rejects absent files, failed hashing or malformed SHA256 tool output.
pub fn binary_sha256(binary: &Path) -> Result<String> {
    let text = crate::process::text(Command::new("sha256sum").arg(binary))?;
    let hash = text.split_whitespace().next().unwrap_or("");
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(failure("invalid binary digest"));
    }
    Ok(hash.to_ascii_lowercase())
}
