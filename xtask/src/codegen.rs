mod object;
pub use object::{find_llvm_tool, readobj_report, symbol_sizes};

use crate::{Error, Result, audit, process};
use serde::Deserialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use uarch_probes::{GATES, Gate};

pub struct Artifacts {
    pub archive: PathBuf,
    pub directory: PathBuf,
    pub ir: PathBuf,
    pub assembly: PathBuf,
    pub object: PathBuf,
}

#[derive(Deserialize)]
struct Message {
    reason: String,
    target: Option<Target>,
    #[serde(default)]
    filenames: Vec<PathBuf>,
}

#[derive(Deserialize)]
struct Target {
    name: String,
}

#[derive(serde::Serialize)]
struct Identity<'a> {
    rustc: &'a str,
    target: &'a str,
    cpu: &'a str,
    profile: &'a str,
    emit: &'a str,
    mca_markers: bool,
    source_sha256: String,
}

fn sha256(bytes: &[u8]) -> Result<String> {
    let mut child = Command::new("sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or_else(|| Error::Gate("missing hash input pipe".into()))?
        .write_all(bytes)?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(Error::Gate("source hashing failed".into()));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let digest = text.split_whitespace().next().unwrap_or("");
    if digest.len() != 64 || !digest.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(Error::Gate("invalid SHA256 digest".into()));
    }
    Ok(digest.to_owned())
}

fn source_hash(root: &Path) -> Result<String> {
    let output = process::output(Command::new("git").arg("-C").arg(root).args([
        "ls-files",
        "--cached",
        "--others",
        "--exclude-standard",
        "-z",
    ]))?;
    let mut names: Vec<_> = output
        .stdout
        .split(|&byte| byte == 0)
        .filter(|name| !name.is_empty())
        .map(|name| {
            String::from_utf8(name.to_vec()).map_err(|error| Error::Gate(error.to_string()))
        })
        .collect::<Result<_>>()?;
    names.sort();
    names.dedup();
    let mut contents = Vec::new();
    for name in names {
        let path = root.join(&name);
        if path.is_file()
            && (name.ends_with(".rs") || name.ends_with(".toml") || name == "Cargo.lock")
        {
            let bytes = std::fs::read(path)?;
            contents.extend_from_slice(&(name.len() as u64).to_le_bytes());
            contents.extend_from_slice(name.as_bytes());
            contents.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
            contents.extend_from_slice(&bytes);
        }
    }
    sha256(&contents)
}

fn version() -> Result<String> {
    process::text(Command::new("rustc").arg("-Vv"))
}

fn host(version: &str) -> Result<&str> {
    version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .ok_or_else(|| Error::Gate("rustc did not report a host triple".into()))
}

fn llvm_major(version: &str) -> Result<u32> {
    let line = version
        .lines()
        .find(|line| line.contains("LLVM version:") || line.contains("LLVM version "))
        .ok_or_else(|| Error::Gate("missing LLVM version".into()))?;
    line.split_whitespace()
        .find_map(|word| word.split('.').next().and_then(|major| major.parse().ok()))
        .ok_or_else(|| Error::Gate("invalid LLVM version".into()))
}

fn llvm_tool(tool: &Path, rustc: &str) -> Result<()> {
    let actual = process::text(Command::new(tool).arg("--version"))?;
    if llvm_major(rustc)? != llvm_major(&actual)? {
        return Err(Error::Gate(format!(
            "{} and rustc must share the same LLVM major version",
            tool.display()
        )));
    }
    Ok(())
}

/// Compiles optimized probe artifacts with LTO disabled for native object emission.
///
/// # Errors
/// Returns an error if compilation or artifact identification fails.
pub fn compile(root: &Path, cpu: &str, markers: bool) -> Result<Artifacts> {
    let rustc = version()?;
    let target = host(&rustc)?;
    let identity = Identity {
        rustc: &rustc,
        target,
        cpu,
        profile: "release; lto=off",
        emit: "llvm-ir,asm,obj,link",
        mca_markers: markers,
        source_sha256: source_hash(root)?,
    };
    let identity = sonic_rs::to_vec(&identity)?;
    let parent = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                root.join(path)
            }
        })
        .unwrap_or_else(|| root.join("target"));
    let directory = parent.join("uarch-gates").join(sha256(&identity)?);
    std::fs::create_dir_all(&directory)?;
    std::fs::write(directory.join("identity.json"), &identity)?;
    let mut command = process::cargo(process::PINNED, root);
    command
        .args([
            "rustc",
            "--locked",
            "-p",
            "uarch-probes",
            "--release",
            "--lib",
            "--message-format=json",
            "--target",
            target,
            "--target-dir",
        ])
        .arg(&directory)
        .env("CARGO_PROFILE_RELEASE_LTO", "off")
        .env(
            "CARGO_ENCODED_RUSTFLAGS",
            if markers {
                format!("-Ctarget-cpu={cpu}\x1f--cfg\x1fuarch_mca")
            } else {
                format!("-Ctarget-cpu={cpu}")
            },
        )
        .args(["--", "--emit=llvm-ir,asm,obj,link"]);
    let output = process::output(command.stdin(Stdio::null()))?;
    let mut artifact = None;
    for line in output
        .stdout
        .split(|&byte| byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let message: Message = sonic_rs::from_slice(line)?;
        if message.reason == "compiler-artifact"
            && message
                .target
                .as_ref()
                .is_some_and(|target| target.name == "uarch_probes")
        {
            artifact = message.filenames.into_iter().find(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "rlib")
            });
        }
    }
    let artifact =
        artifact.ok_or_else(|| Error::Gate("cargo did not report the probe artifact".into()))?;
    let parent = artifact
        .parent()
        .ok_or_else(|| Error::Gate("invalid probe artifact path".into()))?;
    let mut candidates = Vec::new();
    if artifact
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("libuarch_probes-"))
    {
        candidates.push(artifact);
    } else {
        // Cargo can report either the hashed archive or a top-level copy.
        let bytes = std::fs::read(&artifact)?;
        for entry in std::fs::read_dir(parent.join("deps"))? {
            let path = entry?.path();
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("libuarch_probes-") && name.ends_with(".rlib"))
                && std::fs::read(&path)? == bytes
            {
                candidates.push(path);
            }
        }
    }
    if candidates.len() != 1 {
        return Err(Error::Gate(
            "expected one dependency artifact matching Cargo's probe archive".into(),
        ));
    }
    let artifact = &candidates[0];
    let stem = artifact
        .file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem| stem.strip_prefix("lib"))
        .ok_or_else(|| Error::Gate("invalid probe artifact name".into()))?;
    let parent = artifact
        .parent()
        .ok_or_else(|| Error::Gate("invalid probe artifact path".into()))?;
    let artifacts = Artifacts {
        archive: artifact.to_path_buf(),
        directory,
        ir: parent.join(format!("{stem}.ll")),
        assembly: parent.join(format!("{stem}.s")),
        object: parent.join(format!("{stem}.o")),
    };
    for path in [&artifacts.ir, &artifacts.assembly, &artifacts.object] {
        if !path.is_file() {
            return Err(Error::Gate(format!(
                "missing emitted artifact {}",
                path.display()
            )));
        }
    }
    Ok(artifacts)
}

fn selected() -> Vec<Gate> {
    GATES
        .iter()
        .filter(|gate| gate.target_arch == std::env::consts::ARCH)
        .map(|gate| Gate { ..*gate })
        .collect()
}

pub(crate) fn abi(root: &Path) -> Result<()> {
    let artifacts = compile(root, "x86-64", false)?;
    let count = audit::audit_ir(&std::fs::read_to_string(artifacts.ir)?, &selected())?;
    println!("abi-check: {count} concrete signatures passed");
    Ok(())
}

fn objdump() -> Result<PathBuf> {
    Ok(find_llvm_tool("llvm-objdump").unwrap_or_else(|_| PathBuf::from("objdump")))
}

fn verify_objdump(tool: &Path) -> Result<()> {
    if tool
        .file_name()
        .is_some_and(|name| name.to_string_lossy().starts_with("llvm-objdump"))
    {
        llvm_tool(tool, &version()?)
    } else {
        process::output(Command::new(tool).arg("--version"))?;
        Ok(())
    }
}

/// Disassembles a native probe object using an available LLVM or system objdump.
///
/// # Errors
/// Returns an error for unavailable tools, incompatible LLVM, or invalid objects.
pub fn dump(object: &Path) -> Result<String> {
    let tool = objdump()?;
    verify_objdump(&tool)?;
    process::text(
        Command::new(&tool)
            .args(["--disassemble", "--no-show-raw-insn", "--reloc"])
            .arg(object),
    )
}

pub(crate) fn asm(root: &Path) -> Result<()> {
    verify_objdump(&objdump()?)?;
    if std::env::consts::ARCH != "x86_64" {
        return Err(Error::Gate(
            "NotRun: no assembly gate matrix for this host architecture".into(),
        ));
    }
    for cpu in ["x86-64", "x86-64-v3", "x86-64-v4", "znver2", "znver4"] {
        let artifacts = compile(root, cpu, false)?;
        let disassembly = dump(&artifacts.object)?;
        let count = audit::audit_asm(&disassembly, &selected())?;
        object::symbols(&artifacts.object, &selected())?;
        object::sections(&artifacts.object, &selected(), &disassembly)?;
        println!("asm-check: {cpu}: {count} whole-symbol rules passed");
    }
    Ok(())
}

/// Validates an optional comma-separated LLVM CPU list.
///
/// # Errors
/// Rejects empty entries or shell/option punctuation in model names.
pub fn mca_cpu_models(override_list: Option<&str>) -> Result<Vec<String>> {
    let environment = std::env::var("UARCH_MCA_CPU").ok();
    let default = if cfg!(target_arch = "aarch64") {
        "neoverse-v2"
    } else {
        "znver4,znver5,sapphirerapids"
    };
    let mut models = Vec::new();
    for item in override_list
        .or(environment.as_deref())
        .unwrap_or(default)
        .split(',')
    {
        let model = item.trim();
        if model.is_empty()
            || model.starts_with('-')
            || !model
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(Error::Gate("invalid MCA CPU model list".into()));
        }
        if !models.iter().any(|entry| entry == model) {
            models.push(model.to_owned());
        }
    }
    Ok(models)
}

pub(crate) fn mca(root: &Path, override_list: Option<&str>) -> Result<()> {
    let models = mca_cpu_models(override_list)?;
    let rustc = version()?;
    let mca = find_llvm_tool("llvm-mca")?;
    llvm_tool(&mca, &rustc)?;
    verify_objdump(&objdump()?)?;
    let mut checked = 0;
    for target in if cfg!(target_arch = "aarch64") {
        vec!["generic"]
    } else {
        vec!["x86-64", "x86-64-v3", "x86-64-v4"]
    } {
        let gates: Vec<_> = selected()
            .into_iter()
            .filter(|g| {
                g.mca.is_some() && (cfg!(target_arch = "aarch64") || mca_target(g.symbol) == target)
            })
            .collect();
        if gates.is_empty() {
            continue;
        }
        let plain = compile(root, target, false)?;
        let marked = compile(root, target, true)?;
        let plain_dump = dump(&plain.object)?;
        let marked_dump = dump(&marked.object)?;
        let assembly = std::fs::read_to_string(&marked.assembly)?;
        for gate in gates {
            let Some(bound) = gate.mca else { continue };
            if audit::instructions(&plain_dump, gate.symbol)?
                != audit::instructions(&marked_dump, gate.symbol)?
            {
                return Err(Error::Gate(format!(
                    "{}: MCA markers changed machine instructions",
                    gate.symbol
                )));
            }
            let region = audit::mca_region(&assembly, gate.symbol)?;
            let input = marked.ir.with_file_name(format!("{}.mca.s", gate.symbol));
            std::fs::write(
                &input,
                format!(
                    "# LLVM-MCA-BEGIN {}\n{region}# LLVM-MCA-END {}\n",
                    gate.symbol, gate.symbol
                ),
            )?;
            for cpu in &models {
                let report = process::text(
                    Command::new(&mca)
                        .arg("--bottleneck-analysis")
                        .arg(format!("-mcpu={cpu}"))
                        .arg(&input),
                )?;
                let throughput = audit::mca_throughput(&report)?;
                let limit = bound.max_block_reciprocal_throughput
                    * if cpu == bound.cpu { 1.0 } else { 1.25 };
                if throughput > limit {
                    return Err(Error::Gate(format!(
                        "{}/{cpu}: throughput {throughput} exceeds {limit}",
                        gate.symbol
                    )));
                }
                println!(
                    "mca-gate: {} cpu={cpu} target={target} throughput={throughput} budget={limit}",
                    gate.symbol
                );
                checked += 1;
            }
        }
    }
    if checked == 0 {
        return Err(Error::Gate("MCA gate has no selected budgets".into()));
    }
    Ok(())
}

/// Check concrete probe IR bodies, including eliminated invariant division.
///
/// # Errors
/// Returns an error for missing definitions or unreduced arithmetic instructions.
pub fn ir(root: &Path) -> Result<()> {
    let artifacts = compile(root, "x86-64", false)?;
    let source = std::fs::read_to_string(artifacts.ir)?;
    let gates = selected();
    audit::audit_ir(&source, &gates)?;
    for gate in &gates {
        let definition = source
            .lines()
            .position(|line| {
                line.starts_with("define ") && line.contains(&format!("@{}(", gate.symbol))
            })
            .ok_or_else(|| Error::Gate(format!("missing IR definition: {}", gate.symbol)))?;
        let body = source
            .lines()
            .skip(definition + 1)
            .take_while(|line| *line != "}");
        if matches!(gate.symbol, "flow_likely" | "flow_unlikely") {
            let marker = body
                .clone()
                .find_map(|line| line.split_once("!prof !").map(|(_, suffix)| suffix))
                .ok_or_else(|| Error::Gate(format!("{}: missing branch weights", gate.symbol)))?;
            let identifier = marker
                .split(|ch: char| !ch.is_ascii_digit())
                .next()
                .unwrap_or("");
            let metadata = source
                .lines()
                .find(|line| line.starts_with(&format!("!{identifier} = ")))
                .ok_or_else(|| Error::Gate("missing probability metadata".into()))?;
            let weights: Vec<u64> = metadata
                .split("i32 ")
                .skip(1)
                .filter_map(|part| {
                    part.split(|ch: char| !ch.is_ascii_digit())
                        .next()?
                        .parse()
                        .ok()
                })
                .collect();
            if weights.len() != 2
                || (gate.symbol == "flow_likely" && weights[0] <= weights[1])
                || (gate.symbol == "flow_unlikely" && weights[0] >= weights[1])
            {
                return Err(Error::Gate(format!(
                    "{}: wrong probability direction",
                    gate.symbol
                )));
            }
        }
        if gate.symbol.starts_with("fast_divisor_") {
            for line in body {
                if line
                    .split_whitespace()
                    .any(|token| matches!(token, "udiv" | "sdiv" | "urem" | "srem"))
                {
                    return Err(Error::Gate(format!(
                        "{}: unreduced integer division",
                        gate.symbol
                    )));
                }
            }
        }
    }
    println!("ir-check: {} concrete definitions passed", gates.len());
    Ok(())
}

/// Report actual native code sizes for the audited symbols.
///
/// # Errors
/// Returns an error if symbols or nonzero sizes are unavailable.
pub fn size(root: &Path) -> Result<()> {
    let artifacts = compile(root, "x86-64", false)?;
    let sizes = object::symbols(&artifacts.object, &selected())?;
    for (name, size) in sizes {
        println!("size-report: {name}: {size} bytes");
    }
    object::text_size(&artifacts.object)?;
    Ok(())
}

const GEM5_GUEST: &str = r#"#![no_std]
#![no_main]
use core::{arch::{asm, global_asm}, ffi::{c_char,c_void}, hint::black_box, ptr};
#[cfg(target_arch="x86_64")]
global_asm!(".global _start", ".type _start,@function", "_start:", "mov rdi, rsp", "and rsp, -16", "call guest_main", "ud2");
#[cfg(target_arch="aarch64")]
global_asm!(".global _start", ".type _start,%function", "_start:", "mov x0, sp", "bl guest_main", "brk #0");
fn syscall(number:usize,a:usize,b:usize,c:usize)->isize {
    let result:isize;
    // SAFETY: Only write/exit_group are invoked with valid buffers and scalar arguments.
    unsafe {
        #[cfg(target_arch="x86_64")]
        asm!("syscall",inlateout("rax")number=>result,in("rdi")a,in("rsi")b,in("rdx")c,lateout("rcx")_,lateout("r11")_,options(nostack));
        #[cfg(target_arch="aarch64")]
        asm!("svc #0",in("x8")number,inlateout("x0")a=>result,in("x1")b,in("x2")c,options(nostack));
    }
    result
}
fn exit(code:usize)->! {
    syscall(if cfg!(target_arch="x86_64") {231} else {94},code,0,0);
    loop {core::hint::spin_loop();}
}
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>)->! {exit(2)}
#[unsafe(no_mangle)]
extern "C" fn rust_eh_personality() {}
#[unsafe(no_mangle)]
unsafe extern "C" fn strlen(p:*const c_char)->usize {
    let mut n=0;
    // SAFETY: The C caller provides a readable NUL-terminated string.
    unsafe {while ptr::read_volatile(p.add(n))!=0 {n+=1;}}
    n
}
#[unsafe(no_mangle)]
unsafe extern "C" fn memcpy(d:*mut c_void,s:*const c_void,n:usize)->*mut c_void {
    let d=d.cast::<u8>();let s=s.cast::<u8>();
    // SAFETY: The caller provides valid nonoverlapping ranges of n bytes.
    unsafe {for i in 0..n {ptr::write_volatile(d.add(i),ptr::read_volatile(s.add(i)));}}
    d.cast()
}
#[unsafe(no_mangle)]
unsafe extern "C" fn memset(d:*mut c_void,v:i32,n:usize)->*mut c_void {
    let d=d.cast::<u8>();
    // SAFETY: The caller provides a writable range of n bytes.
    unsafe {for i in 0..n {ptr::write_volatile(d.add(i),v as u8);}}
    d.cast()
}
#[unsafe(no_mangle)]
unsafe extern "C" fn memcmp(a:*const c_void,b:*const c_void,n:usize)->i32 {
    let a=a.cast::<u8>();let b=b.cast::<u8>();
    // SAFETY: Both caller-provided ranges contain n readable bytes.
    unsafe {for i in 0..n {let x=ptr::read_volatile(a.add(i));let y=ptr::read_volatile(b.add(i));if x!=y {return i32::from(x)-i32::from(y);}}}
    0
}
#[unsafe(no_mangle)]
unsafe extern "C" fn bcmp(a:*const c_void,b:*const c_void,n:usize)->i32 {
    // SAFETY: The caller provides the same readable ranges required by memcmp.
    unsafe {memcmp(a,b,n)}
}
const NAMES:[&str;8]=["bitword_low_matches","bitword_high_matches","flow_select_register_u64","flow_select_u64","fast_divisor_u64_div","fast_divisor_u32_rem","uarch_arch_scalar_gather","fast_divisor_u64_rem"];
fn run_loop<const KIND:usize>(n:u64)->u64 {
    let d64=uarch_probes::gate::FastDivisorU64::new(core::num::NonZeroU64::MIN.saturating_add(96));
    let d32=uarch_probes::gate::FastDivisorU32::new(core::num::NonZeroU32::MIN.saturating_add(96));
    let mut checksum=0u64;
    for i in 0..n {
        let input=black_box((i&255)|(((i>>3)&7)<<8)|(((i>>4)&15)<<60));
        let value=match KIND {
            0=>u64::from(uarch_probes::bits::bitword_low_matches(input)),
            1=>u64::from(uarch_probes::bits::bitword_high_matches(input)),
            2=>uarch_probes::flow::flow_select_register_u64(black_box(input&1==0),input,black_box(!input)),
            3=>uarch_probes::flow::flow_select_u64(black_box(input&1==0),input,black_box(!input)),
            4=>uarch_probes::arithmetic::fast_divisor_u64_div(black_box(d64),input),
            5=>u64::from(uarch_probes::arithmetic::fast_divisor_u32_rem(black_box(d32),input as u32)),
            7=>uarch_probes::fast_divisor_u64_rem(black_box(d64),input),
            _=>uarch_probes::arch::uarch_arch_scalar_gather(input,black_box(0x5555_5555_5555_5555)),
        };
        checksum=checksum.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(black_box(value)^i.wrapping_add(1).wrapping_mul(0xBF58_476D_1CE4_E5B9));
    }
    checksum
}
// Fixed output width keeps teardown work independent of the checksum value.
fn number(mut n:u64,out:&mut [u8])->usize {
    for i in (0..20).rev() {out[i]=b'0'+(n%10) as u8;n/=10;}
    20
}
unsafe fn argument(p:*const u8)->&'static [u8] {
    let mut len=0;
    // SAFETY: Linux argv supplies readable NUL-terminated strings; cap parser work.
    unsafe {while ptr::read_volatile(p.add(len))!=0 {len+=1;if len>128 {exit(2);}}core::slice::from_raw_parts(p,len)}
}
#[unsafe(no_mangle)]
unsafe extern "C" fn guest_main(stack:*const usize)->! {
    // SAFETY: _start forwards Linux's initial stack, containing argc followed by argv.
    let args=unsafe {core::slice::from_raw_parts(stack.add(1).cast::<*const u8>(),*stack)};
    let mut probe=None;let mut n=None;let mut i=1;
    if args.len()>1 {
        // SAFETY: args contains Linux-provided valid string pointers.
        if unsafe {argument(args[1])}==b"__gem5-probe" {i+=1;}
    }
    while i<args.len() {
        if i+1>=args.len() {exit(2);}
        // SAFETY: Both indices are within the initialized Linux argv array.
        let (key,value)=unsafe {(argument(args[i]),argument(args[i+1]))};
        if key==b"--probe" && probe.is_none() {probe=NAMES.iter().position(|s|s.as_bytes()==value);}
        else if key==b"--iterations" && n.is_none() {
            let mut v=0u64;if value.is_empty() {exit(2);}
            for &ch in value {if !ch.is_ascii_digit() {exit(2);}v=match v.checked_mul(10).and_then(|v|v.checked_add(u64::from(ch-b'0'))) {Some(v)=>v,None=>exit(2)};}
            n=Some(v);
        } else {exit(2);}
        i+=2;
    }
    let (kind,n)=match (probe,n) {(Some(k),Some(n)) if n>0=>(k,n),_=>exit(2)};
    let checksum=match kind {0=>run_loop::<0>(n),1=>run_loop::<1>(n),2=>run_loop::<2>(n),3=>run_loop::<3>(n),4=>run_loop::<4>(n),5=>run_loop::<5>(n),6=>run_loop::<6>(n),_=>run_loop::<7>(n)};
    let mut out=[0u8;160];let mut len=0;
    for part in [b"UARCH_GEM5 probe=".as_slice(),NAMES[kind].as_bytes(),b" n="] {out[len..len+part.len()].copy_from_slice(part);len+=part.len();}
    len+=number(n,&mut out[len..]);out[len..len+10].copy_from_slice(b" checksum=");len+=10;
    len+=number(checksum,&mut out[len..]);out[len]=b'\n';len+=1;
    let mut written=0;
    while written<len {let bytes=syscall(if cfg!(target_arch="x86_64") {1} else {64},1,out[written..].as_ptr() as usize,len-written);if bytes<=0 {exit(2);}written+=bytes as usize;}
    exit(0)
}
"#;

/// Builds a libc-free, baseline-ISA guest against the audited probe archive.
///
/// # Errors
/// Returns an error if artifacts, compilation, or static ELF validation fail.
pub fn gem5_guest(root: &Path) -> Result<PathBuf> {
    let cpu = if cfg!(target_arch = "aarch64") {
        "generic"
    } else {
        "x86-64"
    };
    let artifacts = compile(root, cpu, false)?;
    let directory = &artifacts.directory;
    let source = directory.join(format!("gem5_guest_{}.rs", std::process::id()));
    std::fs::write(&source, GEM5_GUEST)?;
    let binary = directory.join(format!("uarch_gem5_probe_{}", std::process::id()));
    let mut command = Command::new("rustc");
    command
        .args([
            "--edition=2024",
            "-Dwarnings",
            "-O",
            "-C",
            &format!("target-cpu={cpu}"),
            "-C",
            "panic=abort",
            "-C",
            "relocation-model=static",
            "-C",
            "link-arg=-nostdlib",
            "-C",
            "link-arg=-static",
            "-C",
            "link-arg=-Wl,--no-dynamic-linker",
            "--extern",
        ])
        .arg(format!("uarch_probes={}", artifacts.archive.display()));
    let metadata = artifacts.archive.with_extension("rmeta");
    if metadata.is_file() {
        // Recent Cargo separates full metadata from the native archive stub.
        command
            .arg("--extern")
            .arg(format!("uarch_probes={}", metadata.display()));
    }
    let mut pending = vec![directory.clone()];
    while let Some(path) = pending.pop() {
        let mut dependency = false;
        for entry in std::fs::read_dir(&path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                pending.push(entry.path());
            } else if entry
                .path()
                .extension()
                .is_some_and(|ext| matches!(ext.to_str(), Some("rlib" | "rmeta" | "so" | "dylib")))
            {
                dependency = true;
            }
        }
        if dependency {
            command
                .arg("-L")
                .arg(format!("dependency={}", path.display()));
        }
    }
    process::run(command.arg(&source).arg("-o").arg(&binary))?;
    let headers = process::text(Command::new("readelf").args(["-lW"]).arg(&binary))?;
    if headers.contains("INTERP") || headers.contains("DYNAMIC") {
        return Err(Error::Gate(
            "gem5 guest must be a static ELF without PT_INTERP".into(),
        ));
    }
    Ok(binary)
}

/// ISA used to compile a probe before scheduling its instructions in llvm-mca.
pub fn mca_target(symbol: &str) -> &'static str {
    if symbol.starts_with("x86_v4_") {
        "x86-64-v4"
    } else if symbol.starts_with("x86_v3_")
        || matches!(symbol, "uarch_arch_masked_sum" | "uarch_arch_first_ge")
    {
        "x86-64-v3"
    } else {
        "x86-64"
    }
}
