use super::{host, llvm_major, llvm_tool, version};
use crate::{Error, Result, audit, process};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use uarch_probes::{AbiRule, Gate, Instruction};

/// Finds an active-compiler LLVM tool, preferring its sysroot and versioned PATH name.
///
/// # Errors
/// Returns an error if the compiler identity is unavailable or no matching tool exists.
pub fn find_llvm_tool(stem: &str) -> Result<PathBuf> {
    let compiler = version()?;
    let major = llvm_major(&compiler)?;
    let sysroot = process::text(Command::new("rustc").args(["--print", "sysroot"]))?;
    let candidates = [
        Path::new(sysroot.trim())
            .join("lib/rustlib")
            .join(host(&compiler)?)
            .join("bin")
            .join(stem),
        PathBuf::from(format!("{stem}-{major}")),
        PathBuf::from(stem),
    ];
    for candidate in candidates {
        if llvm_tool(&candidate, &compiler).is_ok() {
            return Ok(candidate);
        }
    }
    Err(Error::Gate(format!(
        "NotRun: no {stem} matching LLVM {major}"
    )))
}

/// Parses POSIX nm output and validates unique, nonzero native code symbol sizes.
///
/// # Errors
/// Rejects missing, duplicate, non-code, zero-sized or malformed audited symbols.
pub fn symbol_sizes(table: &str, gates: &[Gate]) -> Result<BTreeMap<String, u64>> {
    let mut sizes = BTreeMap::new();
    for gate in gates {
        let rows: Vec<_> = table
            .lines()
            .map(|line| line.split_whitespace().collect::<Vec<_>>())
            .filter(|fields| fields.first() == Some(&gate.symbol))
            .collect();
        if rows.len() != 1 {
            return Err(Error::Gate(format!(
                "{}: expected one native nm symbol",
                gate.symbol
            )));
        }
        let row = &rows[0];
        let size = row
            .get(3)
            .and_then(|text| u64::from_str_radix(text, 16).ok())
            .filter(|n| *n > 0);
        if row.len() != 4
            || !matches!(row.get(1), Some(&"T" | &"t"))
            || row
                .get(2)
                .and_then(|value| u64::from_str_radix(value, 16).ok())
                .is_none()
            || size.is_none()
        {
            return Err(Error::Gate(format!(
                "{}: invalid native nm symbol size",
                gate.symbol
            )));
        }
        if let Some(size) = size {
            sizes.insert(gate.symbol.to_owned(), size);
        }
    }
    Ok(sizes)
}

pub(super) fn symbols(object: &Path, gates: &[Gate]) -> Result<BTreeMap<String, u64>> {
    let tool = find_llvm_tool("llvm-nm").unwrap_or_else(|_| PathBuf::from("nm"));
    let table = process::text(
        Command::new(&tool)
            .args([
                "-S",
                "--defined-only",
                "--print-size",
                "--size-sort",
                "--format=posix",
            ])
            .arg(object),
    )?;
    println!(
        "symbol-audit: tool={} object={}",
        tool.display(),
        object.display()
    );
    symbol_sizes(&table, gates)
}

fn field_name(value: &str) -> &str {
    value.split(" (").next().unwrap_or(value).trim()
}

fn constant_pool_relocation(line: &str) -> bool {
    let mut fields = line
        .split_whitespace()
        .skip_while(|field| !field.starts_with("R_"));
    let kind = fields.next().unwrap_or("");
    let target = fields
        .next()
        .unwrap_or("")
        .split(['+', '-'])
        .next()
        .unwrap_or("");
    !kind.contains("GOT")
        && !kind.contains("PLT")
        && !matches!(kind, "R_AARCH64_CALL26" | "R_AARCH64_JUMP26")
        && (target == ".rodata" || target.starts_with(".rodata."))
}

/// Validates LLVM readobj sections and rejects relocations in hot leaf symbol ranges.
///
/// # Errors
/// Rejects absent section/symbol evidence, wrong cold placement and hot-leaf relocations.
pub fn readobj_report(report: &str, gates: &[Gate]) -> Result<()> {
    if !report.contains("Sections [") || !report.contains("Relocations [") {
        return Err(Error::Gate(
            "readobj omitted section or relocation evidence".into(),
        ));
    }
    for gate in gates {
        let mut records = Vec::new();
        for block in report.split("  Symbol {").skip(1) {
            let block = block.split("  }").next().unwrap_or("");
            let fields: BTreeMap<_, _> = block
                .lines()
                .filter_map(|line| line.trim().split_once(": "))
                .collect();
            if fields.get("Name").map(|value| field_name(value)) == Some(gate.symbol) {
                records.push(fields);
            }
        }
        if records.len() != 1 {
            return Err(Error::Gate(format!(
                "{}: missing unique readobj symbol",
                gate.symbol
            )));
        }
        let section = records[0]
            .get("Section")
            .map(|value| field_name(value))
            .ok_or_else(|| Error::Gate(format!("{}: missing native section", gate.symbol)))?;
        if let Some(expected) = gate.section
            && section != expected
            && !section.starts_with(&format!("{expected}."))
        {
            return Err(Error::Gate(format!(
                "{}: outside cold section",
                gate.symbol
            )));
        }
        let start = records[0]
            .get("Value")
            .and_then(|v| v.strip_prefix("0x"))
            .and_then(|v| u64::from_str_radix(v, 16).ok())
            .ok_or_else(|| Error::Gate("invalid readobj symbol address".into()))?;
        let size = records[0]
            .get("Size")
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Gate("invalid readobj symbol size".into()))?;
        if gate.section.is_some()
            || !(gate.deny.contains(&Instruction::Call) || gate.abi == AbiRule::Registers)
        {
            continue;
        }
        let mut current = false;
        for line in report.lines() {
            let line = line.trim();
            if line.starts_with("Section (") {
                current = line.contains(&format!(" .rela{section} {{"))
                    || line.contains(&format!(" .rel{section} {{"));
            } else if line == "}" {
                current = false;
            } else if current
                && let Some(offset) = line
                    .split_whitespace()
                    .next()
                    .and_then(|v| v.strip_prefix("0x"))
                    .and_then(|v| u64::from_str_radix(v, 16).ok())
                && offset >= start
                && offset - start < size
                && !constant_pool_relocation(line)
            {
                return Err(Error::Gate(format!(
                    "{}: unexpected hot leaf relocation: {line}",
                    gate.symbol
                )));
            }
        }
    }
    Ok(())
}

pub(super) fn sections(object: &Path, gates: &[Gate], disassembly: &str) -> Result<()> {
    if let Ok(tool) = find_llvm_tool("llvm-readobj") {
        let report = process::text(
            Command::new(&tool)
                .args(["--sections", "--symbols", "--relocations"])
                .arg(object),
        )?;
        std::fs::write(object.with_extension("readobj.txt"), &report)?;
        readobj_report(&report, gates)?;
        println!(
            "section-audit: tool={} cold sections and hot leaf relocations passed",
            tool.display()
        );
    } else {
        // GNU symbol tables carry the same cold-section contract; disassembly retains per-symbol relocation records.
        let table = process::text(Command::new("objdump").arg("--syms").arg(object))?;
        audit::audit_sections(&table, gates)?;
        let report = process::text(Command::new("readelf").args(["-SWsr"]).arg(object))?;
        std::fs::write(object.with_extension("readelf.txt"), report)?;
        for gate in gates.iter().filter(|g| {
            g.section.is_none()
                && (g.deny.contains(&Instruction::Call) || g.abi == AbiRule::Registers)
        }) {
            if audit::instructions(disassembly, gate.symbol)?
                .iter()
                .any(|line| line.contains("R_") && !constant_pool_relocation(line))
            {
                return Err(Error::Gate(format!(
                    "{}: unexpected hot leaf relocation",
                    gate.symbol
                )));
            }
        }
        println!("section-audit: GNU readelf/objdump fallback passed");
    }
    Ok(())
}

pub(super) fn text_size(object: &Path) -> Result<()> {
    let tool = find_llvm_tool("llvm-size").unwrap_or_else(|_| PathBuf::from("size"));
    let summary = process::text(Command::new(&tool).arg(object))?;
    let sections = process::text(Command::new(&tool).arg("--format=sysv").arg(object))?;
    let text = sections
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .filter(|row| {
            row.first()
                .is_some_and(|name| *name == ".text" || name.starts_with(".text."))
        })
        .try_fold(0u64, |total, row| -> Result<u64> {
            let size = row
                .get(1)
                .and_then(|v| v.parse::<u64>().ok())
                .ok_or_else(|| Error::Gate("invalid text section size".into()))?;
            total
                .checked_add(size)
                .ok_or_else(|| Error::Gate("text section size overflow".into()))
        })?;
    if text == 0 {
        return Err(Error::Gate("missing nonzero text section footprint".into()));
    }
    println!(
        "size-report: tool={} total .text footprint={text} bytes\n{summary}",
        tool.display()
    );
    Ok(())
}
