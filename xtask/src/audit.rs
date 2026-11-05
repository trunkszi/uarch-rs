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

/// Raw additive O3 counters; availability preserves missing attribution inputs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Gem5Counters {
    pub num_cycles: f64,
    pub idle_cycles: f64,
    pub committed_insts: f64,
    pub committed_ops: f64,
    pub renamed_insts: f64,
    pub rename_idle_cycles: f64,
    pub rename_squash_cycles: f64,
    pub commit_squashed_insts: f64,
    pub branch_mispredicts: f64,
    pub rob_full_events: f64,
    pub iq_full_events: f64,
    pub lq_full_events: f64,
    pub sq_full_events: f64,
    pub icache_stall_cycles: f64,
    pub dcache_misses: f64,
    pub mem_order_violations: f64,
    pub cpu_prefix: String,
    pub has_rename_slots: bool,
    pub has_structure: bool,
}
impl Gem5Counters {
    fn values(&self) -> [f64; 16] {
        [
            self.num_cycles,
            self.idle_cycles,
            self.committed_insts,
            self.committed_ops,
            self.renamed_insts,
            self.rename_idle_cycles,
            self.rename_squash_cycles,
            self.commit_squashed_insts,
            self.branch_mispredicts,
            self.rob_full_events,
            self.iq_full_events,
            self.lq_full_events,
            self.sq_full_events,
            self.icache_stall_cycles,
            self.dcache_misses,
            self.mem_order_violations,
        ]
    }
    fn with_values(&self, v: &[f64; 16]) -> Self {
        Self {
            num_cycles: v[0],
            idle_cycles: v[1],
            committed_insts: v[2],
            committed_ops: v[3],
            renamed_insts: v[4],
            rename_idle_cycles: v[5],
            rename_squash_cycles: v[6],
            commit_squashed_insts: v[7],
            branch_mispredicts: v[8],
            rob_full_events: v[9],
            iq_full_events: v[10],
            lq_full_events: v[11],
            sq_full_events: v[12],
            icache_stall_cycles: v[13],
            dcache_misses: v[14],
            mem_order_violations: v[15],
            ..self.clone()
        }
    }
}

/// Derived model metrics, explicitly distinct from hardware TMA.
#[derive(Debug)]
pub struct Gem5Metrics {
    pub cycles_per_op: f64,
    pub insts_per_op: f64,
    pub uops_per_op: f64,
    pub ipc: f64,
    pub branch_mispredicts_per_op: f64,
    pub dcache_misses_per_op: f64,
    pub mem_order_violations: f64,
    pub rob_full_per_op: f64,
    pub iq_full_per_op: f64,
    pub lq_full_per_op: f64,
    pub sq_full_per_op: f64,
    pub retiring_pct: f64,
    pub bad_speculation_pct: f64,
    pub frontend_bound_pct: f64,
    pub backend_bound_pct: f64,
    pub dominant_bound: &'static str,
    pub attribution_mode: &'static str,
}

/// Resolves canonical model profiles and inspired-by aliases.
///
/// # Errors
/// Rejects empty elements, unknown profiles and malformed lists.
pub fn gem5_cpu_models(override_list: Option<&str>) -> Result<Vec<String>> {
    let default = if cfg!(target_arch = "aarch64") {
        "o3-a64-w6-rob320"
    } else {
        "o3-w6-rob320,o3-w8-rob448,o3-w6-rob512"
    };
    let mut models = Vec::new();
    for item in override_list.unwrap_or(default).split(',') {
        let model = match item.trim() {
            "o3-w6-rob320" | "zen4-o3" | "znver4" => "o3-w6-rob320",
            "o3-w8-rob448" | "zen5-o3" | "znver5" => "o3-w8-rob448",
            "o3-w6-rob512" | "sapphirerapids-o3" | "sapphirerapids" => "o3-w6-rob512",
            "o3-a64-w6-rob320" | "neoverse-v2-o3" | "neoverse-v2" => "o3-a64-w6-rob320",
            _ => return Err(failure("gem5", "empty or unknown CPU profile")),
        };
        if !models.iter().any(|entry| entry == model) {
            models.push(model.to_owned());
        }
    }
    Ok(models)
}

const GEM5_FIELDS: [&[&str]; 16] = [
    &["numCycles"],
    &["idleCycles"],
    &[
        "commitStats0.numInsts",
        "commit.instsCommitted::total",
        "commit.instsCommitted",
        "committedInsts",
    ],
    &[
        "commitStats0.numOps",
        "commit.opsCommitted::total",
        "commit.opsCommitted",
        "committedOps",
    ],
    &["rename.renamedInsts", "rename.RenamedInsts"],
    &[
        "rename.idleCycles",
        "rename.IdleCycles",
        "rename.status::Idle",
    ],
    &[
        "rename.squashCycles",
        "rename.SquashCycles",
        "rename.status::Squashing",
    ],
    &["commit.commitSquashedInsts", "commit.squashedInsts"],
    &["commit.branchMispredicts", "branchPred.condIncorrect"],
    &["rename.ROBFullEvents"],
    &["rename.IQFullEvents"],
    &["rename.LQFullEvents"],
    &["rename.SQFullEvents"],
    &["fetchStats0.icacheStallCycles", "fetch.icacheStallCycles"],
    &[
        "dcache.overallMisses::total",
        "dcache.overall_misses::total",
    ],
    &["iew.memOrderViolationEvents"],
];

/// Parses a single O3 statistics block, preserving schema and CPU identity.
///
/// # Errors
/// Rejects ambiguous blocks/prefixes, duplicate keys and invalid/missing counters.
pub fn parse_gem5_stats(text: &str) -> Result<Gem5Counters> {
    if text.matches("Begin Simulation Statistics").count() > 1
        || text.matches("Begin Simulation Statistics").count()
            != text.matches("End Simulation Statistics").count()
    {
        return Err(failure("gem5", "ambiguous or truncated statistics blocks"));
    }
    let mut prefix = None;
    let mut stats = std::collections::BTreeMap::new();
    for line in text.lines() {
        let mut words = line.split_whitespace();
        let Some(key) = words.next() else { continue };
        if let Some(cpu) = key.strip_suffix(".numCycles")
            && ![
                "system.cpu",
                "board.processor.cores.core",
                "board.processor.cores0.core",
            ]
            .contains(&cpu)
        {
            return Err(failure("gem5", "unsupported or conflicting CPU prefix"));
        }
        let Some((cpu, suffix)) = [
            "system.cpu",
            "board.processor.cores.core",
            "board.processor.cores0.core",
        ]
        .iter()
        .find_map(|cpu| {
            key.strip_prefix(&format!("{cpu}."))
                .map(|suffix| (*cpu, suffix))
        }) else {
            continue;
        };
        if !GEM5_FIELDS.iter().any(|names| names.contains(&suffix)) {
            continue;
        }
        if prefix.is_some_and(|old| old != cpu) {
            return Err(failure("gem5", "conflicting CPU prefixes"));
        }
        prefix = Some(cpu);
        let value = words
            .next()
            .and_then(|word| word.parse::<f64>().ok())
            .filter(|value| value.is_finite() && *value >= 0.0)
            .ok_or_else(|| failure(key, "invalid raw counter"))?;
        if stats.insert(suffix, value).is_some() {
            return Err(failure(key, "duplicate counter"));
        }
    }
    let mut values = [0.0; 16];
    let mut present = [false; 16];
    for (index, names) in GEM5_FIELDS.iter().enumerate() {
        if let Some(value) = names.iter().find_map(|name| stats.get(name)) {
            values[index] = *value;
            present[index] = true;
        }
    }
    if [0, 2, 3].iter().any(|&i| !present[i]) {
        return Err(failure("gem5", "missing required cycles/insts/ops counter"));
    }
    let base = Gem5Counters {
        cpu_prefix: prefix.unwrap_or("").into(),
        has_rename_slots: present[4] && present[5] && present[6],
        has_structure: present[8] && present[15],
        ..Gem5Counters::default()
    };
    Ok(base.with_values(&values))
}

/// Subtracts raw additive counters without subtracting rates or percentages.
///
/// # Errors
/// Rejects incompatible identities/availability, negative or nonpositive core deltas.
pub fn diff_gem5_counters(high: &Gem5Counters, low: &Gem5Counters) -> Result<Gem5Counters> {
    if high.cpu_prefix != low.cpu_prefix
        || high.has_rename_slots != low.has_rename_slots
        || high.has_structure != low.has_structure
    {
        return Err(failure("gem5", "incompatible counter snapshots"));
    }
    let mut values = [0.0; 16];
    for (i, (hi, lo)) in high.values().into_iter().zip(low.values()).enumerate() {
        values[i] = hi - lo;
        if !hi.is_finite()
            || !lo.is_finite()
            || lo < 0.0
            || values[i] < 0.0
            || !values[i].is_finite()
        {
            return Err(failure(
                GEM5_FIELDS[i][0],
                &format!("negative or invalid counter delta: high={hi} low={lo}"),
            ));
        }
    }
    if [0, 2, 3].iter().any(|&i| values[i] <= 0.0) {
        return Err(failure("gem5", "non-positive cycle/inst/op delta"));
    }
    Ok(high.with_values(&values))
}

/// Validates three-run cycle stationarity and returns the second raw interval.
///
/// # Errors
/// Rejects unordered runs, invalid tolerances and drift beyond the supplied bound.
#[expect(
    clippy::too_many_arguments,
    reason = "The gate contract pairs three snapshots with counts and a tolerance."
)]
pub fn check_gem5_linearity(
    r0: &Gem5Counters,
    n0: u64,
    r1: &Gem5Counters,
    n1: u64,
    r2: &Gem5Counters,
    n2: u64,
    max_rel_drift: f64,
) -> Result<Gem5Counters> {
    if n0 >= n1 || n1 >= n2 || !max_rel_drift.is_finite() || !(0.0..=1.0).contains(&max_rel_drift) {
        return Err(failure("gem5", "invalid linearity arguments"));
    }
    let d1 = diff_gem5_counters(r1, r0)?;
    let d2 = diff_gem5_counters(r2, r1)?;
    let first = d1.num_cycles / (n1 - n0) as f64;
    let second = d2.num_cycles / (n2 - n1) as f64;
    if !first.is_finite()
        || !second.is_finite()
        || first <= 0.0
        || second <= 0.0
        || (second - first).abs() / first > max_rel_drift
    {
        return Err(failure("gem5", "non-stationary O3 ROI"));
    }
    Ok(d2)
}

/// Computes the specified model-specific rename-slot attribution, not hardware TMA.
///
/// # Errors
/// Rejects missing attribution inputs, invalid denominators or inconsistent slots.
pub fn compute_gem5_metrics(
    d: &Gem5Counters,
    operations: u64,
    slot_width: u64,
) -> Result<Gem5Metrics> {
    if operations == 0
        || slot_width == 0
        || !d.has_rename_slots
        || d.values().iter().any(|v| !v.is_finite() || *v < 0.0)
        || d.num_cycles <= 0.0
        || d.idle_cycles > d.num_cycles
    {
        return Err(failure("gem5", "invalid or missing O3 metric inputs"));
    }
    let n = operations as f64;
    let slots = (d.num_cycles - d.idle_cycles).max(1.0) * slot_width as f64;
    let retiring = 100.0 * d.committed_ops / slots;
    let bad = 100.0
        * ((d.renamed_insts - d.committed_ops).max(0.0)
            + slot_width as f64 * d.rename_squash_cycles)
        / slots;
    let frontend = 100.0 * slot_width as f64 * d.rename_idle_cycles / slots;
    let sum = retiring + bad + frontend;
    if !sum.is_finite() || sum > 105.0 {
        return Err(failure("gem5", "inconsistent O3 slot accounting"));
    }
    let backend = (100.0 - sum).clamp(0.0, 100.0);
    let buckets = [retiring, bad, frontend, backend];
    let names = [
        "Retiring",
        "Bad Speculation",
        "Frontend Bound",
        "Backend Bound",
    ];
    let mut dominant = 0;
    for i in 1..4 {
        if buckets[i] > buckets[dominant] {
            dominant = i;
        }
    }
    Ok(Gem5Metrics {
        cycles_per_op: d.num_cycles / n,
        insts_per_op: d.committed_insts / n,
        uops_per_op: d.committed_ops / n,
        ipc: d.committed_insts / d.num_cycles,
        branch_mispredicts_per_op: d.branch_mispredicts / n,
        dcache_misses_per_op: d.dcache_misses / n,
        mem_order_violations: d.mem_order_violations,
        rob_full_per_op: d.rob_full_events / n,
        iq_full_per_op: d.iq_full_events / n,
        lq_full_per_op: d.lq_full_events / n,
        sq_full_per_op: d.sq_full_events / n,
        retiring_pct: retiring,
        bad_speculation_pct: bad,
        frontend_bound_pct: frontend,
        backend_bound_pct: backend,
        dominant_bound: names[dominant],
        attribution_mode: "o3-rename-slot",
    })
}
