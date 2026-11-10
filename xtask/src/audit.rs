use crate::{Error, Result};
use uarch_probes::{AbiRule, Gate, Instruction};

fn failure(symbol: &str, detail: &str) -> Error {
    Error::Gate(format!("{symbol}: {detail}"))
}

fn ir_name(signature: &str) -> Option<&str> {
    let after = signature.split_once('@')?.1;
    if let Some(quoted) = after.strip_prefix('"') {
        Some(quoted.split_once('"')?.0)
    } else {
        Some(after.split_once('(')?.0.trim())
    }
}

/// Checks concrete definitions, including a by-value memory negative control.
///
/// # Errors
/// Returns an error for absent, duplicate, malformed, or nonconforming signatures.
pub fn audit_ir(ir: &str, gates: &[Gate]) -> Result<usize> {
    let mut definitions = Vec::new();
    let mut lines = ir.lines();
    while let Some(line) = lines.next() {
        if !line.trim_start().starts_with("define ") {
            continue;
        }
        let mut signature = line.trim().to_owned();
        while !signature.trim_end().ends_with('{') {
            let next = lines
                .next()
                .ok_or_else(|| failure("IR", "unterminated definition"))?;
            signature.push(' ');
            signature.push_str(next.trim());
        }
        definitions.push(signature);
    }
    let mut checked = 0;
    for gate in gates.iter().filter(|gate| gate.abi != AbiRule::Unchecked) {
        let matching: Vec<_> = definitions
            .iter()
            .filter(|signature| ir_name(signature) == Some(gate.symbol))
            .collect();
        if matching.len() != 1 {
            return Err(failure(gate.symbol, "expected exactly one LLVM definition"));
        }
        let signature = matching[0];
        let memory = signature
            .split(|c: char| c.is_whitespace() || matches!(c, ',' | '(' | ')'))
            .any(|word| {
                word == "sret"
                    || word.starts_with("sret(")
                    || word == "byval"
                    || word.starts_with("byval(")
                    || word == "ptr"
                    || word == "dereferenceable"
                    || word.starts_with("dereferenceable(")
                    || word == "dead_on_return"
            });
        match gate.abi {
            AbiRule::Registers if memory => {
                return Err(failure(
                    gate.symbol,
                    "indirect value-passing memory fallback",
                ));
            }
            AbiRule::MemoryControl
                if !signature
                    .split(|c: char| c.is_whitespace() || matches!(c, ',' | '(' | ')'))
                    .any(|word| word == "byval" || word.starts_with("byval(")) =>
            {
                return Err(failure(gate.symbol, "negative control lacks byval"));
            }
            _ => checked += 1,
        }
    }
    if checked == 0 {
        return Err(failure("IR", "no ABI rules selected"));
    }
    Ok(checked)
}

fn symbol_header(line: &str) -> Option<&str> {
    let line = line.trim();
    let (address, rest) = line.split_once(" <")?;
    if !address.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    rest.strip_suffix(">:")
}

/// Extracts complete symbol bodies from objdump's no-raw-bytes format.
///
/// # Errors
/// Returns an error for absent or ambiguous symbols and empty bodies.
pub fn instructions(dump: &str, symbol: &str) -> Result<Vec<String>> {
    let mut body: Vec<String> = Vec::new();
    let mut active = false;
    let mut count = 0;
    for line in dump.lines() {
        if let Some(name) = symbol_header(line) {
            active = name == symbol;
            count += usize::from(active);
        } else if active && let Some((address, instruction)) = line.trim().split_once(':') {
            if address.is_empty() || !address.chars().all(|c| c.is_ascii_hexdigit()) {
                continue;
            }
            let instruction = instruction.trim();
            if instruction.starts_with("R_") {
                if let Some(previous) = body.last_mut() {
                    previous.push_str(" ; ");
                    previous.push_str(instruction);
                }
            } else if !instruction.is_empty() {
                body.push(instruction.to_owned());
            }
        }
    }
    if count != 1 || body.is_empty() {
        return Err(failure(
            symbol,
            "expected exactly one nonempty disassembled symbol",
        ));
    }
    Ok(body)
}

fn tail_jump(line: &str, symbol: &str, local: &[u64]) -> bool {
    if line.contains("R_") {
        return true;
    }
    let operand = line
        .split_whitespace()
        .skip_while(|word| !word.starts_with("jmp") && !matches!(*word, "b" | "br"))
        .nth(1)
        .unwrap_or("");
    if let Some((_, target)) = line.split_once('<') {
        let target = target.split('>').next().unwrap_or("");
        return target != symbol
            && !target
                .strip_prefix(symbol)
                .is_some_and(|rest| rest.starts_with("+0x"));
    }
    if operand.starts_with(".L")
        || operand.strip_suffix(['f', 'b']).is_some_and(|number| {
            !number.is_empty() && number.chars().all(|ch| ch.is_ascii_digit())
        })
    {
        return false;
    }
    let numeric = operand.strip_prefix("0x").unwrap_or(operand);
    !u64::from_str_radix(numeric, 16)
        .ok()
        .is_some_and(|target| local.contains(&target))
}

fn mnemonic(line: &str) -> &str {
    line.split_whitespace()
        .find(|word| {
            ![
                "rep", "repe", "repz", "repne", "repnz", "lock", "bnd", "notrack", "data16",
                "addr32",
            ]
            .contains(word)
        })
        .unwrap_or("")
}

fn matches_instruction(line: &str, instruction: Instruction, symbol: &str, local: &[u64]) -> bool {
    let mnemonic = mnemonic(line);
    match instruction {
        Instruction::Jcc => {
            (mnemonic.starts_with('j') && !mnemonic.starts_with("jmp"))
                || mnemonic.starts_with("loop")
                || mnemonic.starts_with("b.")
                || matches!(mnemonic, "cbz" | "cbnz" | "tbz" | "tbnz")
        }
        Instruction::Movabs => mnemonic.starts_with("movabs"),
        Instruction::Call => {
            mnemonic.starts_with("call")
                || matches!(mnemonic, "bl" | "blr")
                || ((mnemonic.starts_with("jmp") || matches!(mnemonic, "b" | "br"))
                    && tail_jump(line, symbol, local))
        }
        Instruction::Div => matches!(
            mnemonic,
            "div"
                | "udiv"
                | "sdiv"
                | "divb"
                | "divw"
                | "divl"
                | "divq"
                | "idiv"
                | "idivb"
                | "idivw"
                | "idivl"
                | "idivq"
        ),
        Instruction::Idiv => matches!(
            mnemonic,
            "idiv" | "idivb" | "idivw" | "idivl" | "idivq" | "sdiv"
        ),
        Instruction::Imul => matches!(
            mnemonic,
            "imul"
                | "imulb"
                | "imulw"
                | "imull"
                | "imulq"
                | "mul"
                | "mulb"
                | "mulw"
                | "mull"
                | "mulq"
                | "mulx"
                | "mulxq"
                | "mulxl"
                | "umulh"
                | "madd"
                | "msub"
        ),
        Instruction::Udivti3 => line.contains("__udivti3"),
        Instruction::Pext => mnemonic.starts_with("pext"),
        Instruction::Pdep => mnemonic.starts_with("pdep"),
    }
}

/// Checks instruction budgets and required/forbidden opcodes per whole symbol.
///
/// # Errors
/// Returns an error if any symbol or instruction contract is violated.
pub fn audit_asm(dump: &str, gates: &[Gate]) -> Result<usize> {
    if gates.is_empty() {
        return Err(failure("ASM", "no instruction rules selected"));
    }
    for gate in gates {
        let body = instructions(dump, gate.symbol)?;
        let mut active = false;
        let local: Vec<u64> = dump
            .lines()
            .filter_map(|line| {
                if let Some(name) = symbol_header(line) {
                    active = name == gate.symbol;
                    return None;
                }
                if active {
                    line.trim()
                        .split_once(':')
                        .and_then(|(address, _)| u64::from_str_radix(address, 16).ok())
                } else {
                    None
                }
            })
            .collect();
        if gate.abi == AbiRule::Registers
            && body.iter().any(|line| {
                let lower = line.to_ascii_lowercase().replace(' ', "");
                let mnemonic = mnemonic(line);
                matches!(
                    mnemonic,
                    "push"
                        | "pushq"
                        | "pushl"
                        | "pushw"
                        | "pushf"
                        | "pushfq"
                        | "pop"
                        | "popq"
                        | "popl"
                        | "popw"
                        | "popf"
                        | "popfq"
                ) || ["(%rsp", "(%rbp", "[rsp", "[rbp", "[sp"]
                    .iter()
                    .any(|pattern| lower.contains(pattern))
            })
        {
            return Err(failure(
                gate.symbol,
                "register leaf uses a stack frame or spill",
            ));
        }
        if gate.max_instructions.is_some_and(|max| body.len() > max) {
            return Err(failure(gate.symbol, "instruction budget exceeded"));
        }
        for &instruction in gate.deny {
            if body
                .iter()
                .any(|line| matches_instruction(line, instruction, gate.symbol, &local))
            {
                return Err(failure(gate.symbol, &format!("forbidden {instruction:?}")));
            }
        }
        for &instruction in gate.require {
            if !body
                .iter()
                .any(|line| matches_instruction(line, instruction, gate.symbol, &local))
            {
                return Err(failure(gate.symbol, &format!("missing {instruction:?}")));
            }
        }
    }
    Ok(gates.len())
}

/// Checks symbol-table section membership rather than assembly text spelling.
///
/// # Errors
/// Returns an error if a section-constrained symbol is absent or misplaced.
pub fn audit_sections(table: &str, gates: &[Gate]) -> Result<()> {
    for gate in gates {
        if let Some(section) = gate.section {
            let rows: Vec<_> = table
                .lines()
                .filter(|line| line.split_whitespace().last() == Some(gate.symbol))
                .collect();
            if rows.len() != 1
                || !rows[0]
                    .split_whitespace()
                    .any(|word| word == section || word.starts_with(&format!("{section}.")))
            {
                return Err(failure(gate.symbol, "symbol is outside required section"));
            }
        }
    }
    Ok(())
}

/// Extracts one named LLVM-MCA marked interval from compiler-generated assembly.
///
/// # Errors
/// Returns an error for absent, repeated, nested, or unterminated intervals.
pub fn mca_region(assembly: &str, symbol: &str) -> Result<String> {
    let begin = format!("LLVM-MCA-BEGIN {symbol}");
    let end = format!("LLVM-MCA-END {symbol}");
    if !assembly.contains(&begin) && !assembly.contains(&end) {
        let mut active = false;
        let mut complete = false;
        let mut body = String::new();
        let mut count = 0;
        for line in assembly.lines() {
            let text = line.split('#').next().unwrap_or("").trim();
            if text == format!("{symbol}:") || text == format!("\"{symbol}\":") {
                count += 1;
                active = true;
            } else if active && (text.starts_with(".Lfunc_end") || text.starts_with(".size")) {
                active = false;
                complete = true;
            } else if active && !text.is_empty() && (!text.starts_with('.') || text.ends_with(':'))
            {
                body.push_str(line);
                body.push('\n');
            }
        }
        if count != 1 || !complete || body.trim().is_empty() {
            return Err(failure(symbol, "missing or incomplete assembly function"));
        }
        return Ok(body);
    }
    let mut active = false;
    let mut complete = false;
    let mut body = String::new();
    for line in assembly.lines() {
        if line.trim().trim_start_matches(['#', '/']).trim() == begin {
            if active || complete {
                return Err(failure(symbol, "duplicate MCA region"));
            }
            active = true;
        } else if line.trim().trim_start_matches(['#', '/']).trim() == end {
            if !active {
                return Err(failure(symbol, "unmatched MCA end marker"));
            }
            active = false;
            complete = true;
        } else if active && !line.trim_start().starts_with(".prefalign") {
            if line.contains("LLVM-MCA-") {
                return Err(failure(symbol, "nested MCA region"));
            }
            body.push_str(line);
            body.push('\n');
        }
    }
    if active || !complete || body.trim().is_empty() {
        return Err(failure(symbol, "missing or incomplete MCA region"));
    }
    Ok(body)
}

/// Reads a finite nonnegative Block RThroughput value from LLVM-MCA output.
///
/// # Errors
/// Returns an error for absent, repeated, or invalid throughput measurements.
pub fn mca_throughput(report: &str) -> Result<f64> {
    let values: Vec<_> = report
        .lines()
        .filter_map(|line| line.trim().strip_prefix("Block RThroughput:"))
        .collect();
    if values.len() != 1 {
        return Err(failure("MCA", "expected one throughput measurement"));
    }
    let value: f64 = values[0]
        .trim()
        .parse()
        .map_err(|_| failure("MCA", "invalid throughput"))?;
    if !value.is_finite() || value < 0.0 {
        return Err(failure("MCA", "invalid throughput"));
    }
    Ok(value)
}

/// Position-sensitive, wrapping checksum shared by guest and host probes.
pub const fn combine_probe_checksum(acc: u64, index: u64, value: u64) -> u64 {
    acc.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(value ^ index.wrapping_add(1).wrapping_mul(0xBF58_476D_1CE4_E5B9))
}

pub use uarch_audit::gem5_stats::{Gem5Counters, Gem5Metrics};
/// Resolves the shared supported O3 profiles.
/// # Errors
/// Rejects malformed CPU profile declarations.
pub fn gem5_cpu_models(list: Option<&str>) -> Result<Vec<String>> {
    uarch_audit::gem5_stats::gem5_cpu_models(list).map_err(Error::Gate)
}
/// Parses shared additive O3 counters.
/// # Errors
/// Rejects ambiguous or malformed statistics.
pub fn parse_gem5_stats(text: &str) -> Result<Gem5Counters> {
    uarch_audit::gem5_stats::parse_gem5_stats(text).map_err(Error::Gate)
}
/// Subtracts compatible additive O3 counters.
/// # Errors
/// Rejects incompatible or invalid counter deltas.
pub fn diff_gem5_counters(high: &Gem5Counters, low: &Gem5Counters) -> Result<Gem5Counters> {
    uarch_audit::gem5_stats::diff_gem5_counters(high, low).map_err(Error::Gate)
}
/// Checks adjacent interval stationarity.
/// # Errors
/// Rejects unordered steps or non-stationary measurements.
#[expect(
    clippy::too_many_arguments,
    reason = "Each of three measurements pairs additive counters with its operation denominator."
)]
pub fn check_gem5_linearity(
    a: &Gem5Counters,
    na: u64,
    b: &Gem5Counters,
    nb: u64,
    c: &Gem5Counters,
    nc: u64,
    tolerance: f64,
) -> Result<Gem5Counters> {
    uarch_audit::gem5_stats::check_gem5_linearity(a, na, b, nb, c, nc, tolerance)
        .map_err(Error::Gate)
}
/// Computes model rename-slot attribution, not hardware TMA.
/// # Errors
/// Rejects invalid denominators or missing model counters.
pub fn compute_gem5_metrics(d: &Gem5Counters, n: u64, width: u64) -> Result<Gem5Metrics> {
    uarch_audit::gem5_stats::compute_gem5_metrics(d, n, width).map_err(Error::Gate)
}
