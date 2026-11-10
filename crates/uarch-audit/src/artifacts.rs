//! Cargo identity selects outputs; directory timestamps never select evidence.
use crate::{Error, Result};
use std::path::{Path, PathBuf};

fn failure(detail: impl std::fmt::Display) -> Error {
    Error::Gate(format!("[audit.artifacts] {detail}"))
}
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CargoArtifact {
    pub manifest_path: PathBuf,
    pub filenames: Vec<PathBuf>,
    pub executable: Option<PathBuf>,
}
#[derive(serde::Deserialize)]
struct Message {
    reason: String,
    manifest_path: Option<PathBuf>,
    target: Option<Target>,
    #[serde(default)]
    filenames: Vec<PathBuf>,
    executable: Option<PathBuf>,
}
#[derive(serde::Deserialize)]
struct Target {
    name: String,
}
/// Selects exactly one compiler-artifact by package manifest and target identity.
/// # Errors
/// Rejects malformed JSON, failed builds, absent or ambiguous target records.
pub fn select_artifact(messages: &str, manifest: &Path, target: &str) -> Result<CargoArtifact> {
    let mut selected = None;
    for line in messages.lines().filter(|l| !l.trim().is_empty()) {
        let value: sonic_rs::Value = sonic_rs::from_str(line)?;
        use sonic_rs::JsonValueTrait;
        if value.get("reason").and_then(|v| v.as_str()) == Some("build-finished")
            && value.get("success").and_then(|v| v.as_bool()) != Some(true)
        {
            return Err(failure("Cargo build failed"));
        }
        let msg: Message = sonic_rs::from_str(line)?;
        if msg.reason == "compiler-artifact"
            && msg.manifest_path.as_deref() == Some(manifest)
            && msg.target.is_some_and(|t| t.name == target)
        {
            if selected.is_some() {
                return Err(failure("ambiguous compiler-artifact"));
            }
            selected = Some(CargoArtifact {
                manifest_path: manifest.to_owned(),
                filenames: msg.filenames,
                executable: msg.executable,
            });
        }
    }
    selected.ok_or_else(|| failure("missing compiler-artifact for manifest/target"))
}
#[derive(Debug, Clone)]
pub struct EmittedArtifacts {
    pub object: PathBuf,
    pub ir: PathBuf,
    pub assembly: PathBuf,
}
impl CargoArtifact {
    /// Resolves emitted sidecars using only Cargo-reported hashed artifact stems.
    /// # Errors
    /// Rejects missing/ambiguous emissions and non-files. An unhashed Cargo copy cannot identify sidecars.
    pub fn emitted(&self) -> Result<EmittedArtifacts> {
        let path_for = |extension: &str| -> Result<PathBuf> {
            let mut candidates: Vec<_> = self
                .filenames
                .iter()
                .filter(|p| p.extension().is_some_and(|e| e == extension))
                .cloned()
                .collect();
            if candidates.is_empty() {
                for archive in &self.filenames {
                    if archive.extension().is_some_and(|e| e == "rlib")
                        && let Some(stem) = archive
                            .file_stem()
                            .and_then(|n| n.to_str())
                            .and_then(|n| n.strip_prefix("lib"))
                    {
                        let candidate = archive.with_file_name(format!("{stem}.{extension}"));
                        if candidate.is_file() {
                            candidates.push(candidate);
                        }
                    }
                }
            }
            candidates.sort();
            candidates.dedup();
            if candidates.len() != 1 || !candidates[0].is_file() {
                return Err(failure(format!(
                    "expected exactly one emitted .{extension}"
                )));
            }
            Ok(candidates.remove(0))
        };
        Ok(EmittedArtifacts {
            object: path_for("o")?,
            ir: path_for("ll")?,
            assembly: path_for("s")?,
        })
    }
}
/// Ensures each exact cold symbol is assigned an unlikely text section.
/// # Errors
/// Rejects absent, duplicate or incorrectly placed symbols in objdump's symbol table.
pub fn verify_cold_sections(table: &str, symbols: &[String]) -> Result<()> {
    for symbol in symbols {
        let rows: Vec<_> = table
            .lines()
            .map(|l| l.split_whitespace().collect::<Vec<_>>())
            .filter(|f| f.last() == Some(&symbol.as_str()))
            .collect();
        if rows.len() != 1
            || !rows[0]
                .iter()
                .any(|s| *s == ".text.unlikely" || s.starts_with(".text.unlikely."))
        {
            return Err(failure(format!(
                "{symbol}: missing unique .text.unlikely placement"
            )));
        }
    }
    Ok(())
}
fn symbol_body(dump: &str, symbol: &str) -> Result<Vec<String>> {
    let mut found = 0;
    let mut active = false;
    let mut body = Vec::new();
    for line in dump.lines() {
        let line = line.trim();
        if let Some((address, tail)) = line.split_once(" <")
            && address.chars().all(|c| c.is_ascii_hexdigit())
            && let Some(name) = tail.strip_suffix(">:")
        {
            active = name == symbol;
            found += usize::from(active);
        } else if active
            && let Some((pc, instruction)) = line.split_once(':')
            && !pc.is_empty()
            && pc.chars().all(|c| c.is_ascii_hexdigit())
            && !instruction.trim().is_empty()
        {
            body.push(instruction.trim().to_owned());
        }
    }
    if found != 1 || body.is_empty() {
        return Err(failure(format!(
            "{symbol}: missing or duplicate native symbol"
        )));
    }
    Ok(body)
}
// Literal text and comments cannot establish an LLVM execution contract.
fn unquoted_ir(line: &str, group_only: bool) -> String {
    let mut visible = String::with_capacity(line.len());
    let mut quoted = false;
    let mut escaped = false;
    let mut inside = !group_only;
    for ch in line.chars() {
        if quoted {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quoted = false;
            }
            continue;
        }
        match ch {
            ';' => break,
            '"' => {
                quoted = true;
                if inside {
                    visible.push(' ');
                }
            }
            '{' if group_only => {
                inside = true;
                visible.push(' ');
            }
            '}' if group_only => break,
            _ if inside => visible.push(if matches!(ch, '(' | ')' | '{' | '}' | ',') {
                ' '
            } else {
                ch
            }),
            _ => {}
        }
    }
    visible
}

/// Checks all instructions in an exact hot symbol and its LLVM unwind/division attributes.
/// # Errors
/// Rejects traps, integer division, exception handling, uwtable, or absent/ambiguous definitions.
pub fn verify_hot_symbol(dump: &str, ir: &str, symbol: &str, allow_division: bool) -> Result<()> {
    for instruction in symbol_body(dump, symbol)? {
        let words: Vec<_> = instruction.split_whitespace().collect();
        let op = words
            .iter()
            .find(|w| {
                !matches!(
                    **w,
                    "lock"
                        | "rep"
                        | "repe"
                        | "repz"
                        | "repne"
                        | "repnz"
                        | "notrack"
                        | "bnd"
                        | "cs"
                        | "ds"
                        | "ss"
                        | "es"
                        | "fs"
                        | "gs"
                        | "data16"
                        | "addr16"
                        | "addr32"
                        | "rex"
                        | "rex.W"
                        | "rex64"
                )
            })
            .copied()
            .unwrap_or("");
        if matches!(op, "ud2" | "ud2a" | "udf" | "brk" | "int3" | "hlt")
            || (!allow_division
                && matches!(
                    op,
                    "div"
                        | "divb"
                        | "divw"
                        | "divl"
                        | "divq"
                        | "idiv"
                        | "idivb"
                        | "idivw"
                        | "idivl"
                        | "idivq"
                        | "udiv"
                        | "sdiv"
                ))
        {
            return Err(failure(format!("{symbol}: forbidden instruction {op}")));
        }
    }
    let mut definitions = 0;
    let mut active = false;
    let mut attribute = None;
    let mut has_personality = false;
    let mut nounwind = false;
    for line in ir.lines() {
        let visible = unquoted_ir(line, false);
        let words: Vec<_> = visible.split_whitespace().collect();
        if line.starts_with("define ") {
            active =
                line.contains(&format!("@{symbol}(")) || line.contains(&format!("@\"{symbol}\"("));
            definitions += usize::from(active);
            if active {
                has_personality = words.contains(&"personality");
                nounwind = words.contains(&"nounwind");
                attribute = words
                    .iter()
                    .find(|word| {
                        word.strip_prefix('#')
                            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
                    })
                    .map(|word| (*word).to_owned());
            }
        }
        if active {
            if words.iter().any(|w| {
                matches!(*w, "invoke" | "landingpad" | "resume" | "uwtable")
                    || (*w == "personality" && !line.starts_with("define "))
                    || (!allow_division && matches!(*w, "sdiv" | "udiv" | "srem" | "urem"))
            }) || line.contains("_Unwind_Resume")
            {
                return Err(failure(format!(
                    "{symbol}: unwind or integer division in IR"
                )));
            }
            if line.trim_start().starts_with('}') {
                active = false;
            }
        }
    }
    if definitions != 1 {
        return Err(failure(format!(
            "{symbol}: missing or duplicate LLVM definition"
        )));
    }
    if let Some(attribute) = attribute {
        let prefix = format!("attributes {attribute} =");
        let matching: Vec<_> = ir.lines().filter(|l| l.starts_with(&prefix)).collect();
        if matching.len() != 1 {
            return Err(failure(format!("{symbol}: missing attribute group")));
        }
        let visible = unquoted_ir(matching[0], true);
        let words: Vec<_> = visible.split_whitespace().collect();
        if words.contains(&"uwtable") {
            return Err(failure(format!("{symbol}: unwind table")));
        }
        nounwind |= words.contains(&"nounwind");
    }
    // LLVM can retain a personality on abort-only inline-asm functions.
    // Only nounwind plus the absence of actual EH operations justifies it.
    if has_personality && !nounwind {
        return Err(failure(format!("{symbol}: personality without nounwind")));
    }
    Ok(())
}
