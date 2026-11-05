//! Read-only host evidence for capability detection.
//!
//! # Deployment & Thread Pinning
//! Multi-CPU x86_64 Linux auditing needs readable `/dev/cpu/N/cpuid` devices for
//! every allowed CPU. Permissions are deployment-dependent, often restricted to
//! root or privileged environments; granting `CAP_SYS_RAWIO` alone does not
//! replace device-file read permissions. Missing evidence is [`Observed::Unknown`].
//! Pin latency-sensitive worker threads with `sched_setaffinity` or
//! `taskset -c <cpu>` before calling [`crate::detect`] so singleton CPUID/XGETBV
//! verification can avoid these devices. This auditor never changes affinity.
//! See [`crate::detect`] for Scalar fallback and strict-domain error semantics.

#[cfg(target_arch = "x86_64")]
use crate::{CpuidSource, HardwareCpuid};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Observed<T> {
    Known(T),
    Unknown,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheLine {
    pub cpu: u32,
    pub level: u8,
    pub bytes: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HugePageConfig {
    pub size_kib: u64,
    pub total: u64,
    pub free: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmtSiblings {
    pub cpu: u32,
    pub siblings: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostReport {
    pub affinity_cpus: Observed<Vec<u32>>,
    pub isa_homogeneous: Observed<bool>,
    pub cache_lines: Observed<Vec<CacheLine>>,
    pub smt_active: Observed<bool>,
    pub smt_siblings: Observed<Vec<SmtSiblings>>,
    pub huge_pages: Observed<HugePageConfig>,
    pub invariant_tsc: Observed<bool>,
}

pub struct HostHardwareAudit;

impl HostHardwareAudit {
    /// Observes host evidence without changing affinity, huge pages or CPU state.
    pub fn probe() -> HostReport {
        let cpus = affinity_cpus();
        HostReport {
            affinity_cpus: option_observed(cpus.clone()),
            isa_homogeneous: isa_homogeneous(),
            cache_lines: option_observed(cpus.as_deref().and_then(cache_lines)),
            smt_active: option_observed(
                read_number("/sys/devices/system/cpu/smt/active").map(|n| n != 0),
            ),
            smt_siblings: option_observed(cpus.as_deref().and_then(smt_siblings)),
            huge_pages: option_observed(huge_pages()),
            invariant_tsc: invariant_tsc(cpus.as_deref()),
        }
    }
}

fn option_observed<T>(value: Option<T>) -> Observed<T> {
    match value {
        Some(value) => Observed::Known(value),
        None => Observed::Unknown,
    }
}

pub(crate) fn affinity_cpus() -> Option<Vec<u32>> {
    #[cfg(target_os = "linux")]
    {
        unsafe extern "C" {
            fn sched_getaffinity(
                pid: core::ffi::c_int,
                size: usize,
                mask: *mut core::ffi::c_void,
            ) -> core::ffi::c_int;
        }
        // The kernel may have a CPU mask larger than libc's fixed cpu_set_t.
        for bytes in [128, 256, 512, 1024, 2048, 4096, 8192] {
            let mut mask = vec![0usize; bytes / core::mem::size_of::<usize>()];
            // SAFETY: The vector owns bytes writable bytes aligned as machine words.
            let rc = unsafe { sched_getaffinity(0, bytes, mask.as_mut_ptr().cast()) };
            if rc == 0 {
                let mut cpus = Vec::new();
                for (word_index, word) in mask.into_iter().enumerate() {
                    for bit in 0..usize::BITS {
                        if word & (1usize << bit) != 0 {
                            cpus.push((word_index * usize::BITS as usize + bit as usize) as u32);
                        }
                    }
                }
                return (!cpus.is_empty()).then_some(cpus);
            }
            if std::io::Error::last_os_error().raw_os_error() != Some(22) {
                return None;
            }
        }
        None
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(target_arch = "x86_64")]
pub(crate) fn execution_domain() -> Option<Vec<crate::cpuid::Snapshot>> {
    let before = affinity_cpus()?;
    let sources = if before.len() == 1 {
        vec![crate::cpuid::Snapshot::from_source(&HardwareCpuid)]
    } else {
        #[cfg(target_os = "linux")]
        {
            let xcr0 = HardwareCpuid.xcr0();
            before
                .iter()
                .map(|cpu| remote_snapshot(*cpu, xcr0))
                .collect::<Option<Vec<_>>>()?
        }
        #[cfg(not(target_os = "linux"))]
        {
            return None;
        }
    };
    // A changed domain invalidates evidence gathered against the earlier mask.
    (affinity_cpus()? == before).then_some(sources)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn remote_registers(file: &std::fs::File, leaf: u32) -> Option<crate::CpuidRegisters> {
    use std::os::unix::fs::FileExt;
    let mut bytes = [0; 16];
    file.read_exact_at(&mut bytes, u64::from(leaf)).ok()?;
    let mut r = [0u32; 4];
    for (value, chunk) in r.iter_mut().zip(bytes.as_chunks::<4>().0) {
        *value = u32::from_le_bytes(*chunk);
    }
    Some(crate::CpuidRegisters {
        eax: r[0],
        ebx: r[1],
        ecx: r[2],
        edx: r[3],
    })
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn remote_snapshot(cpu: u32, xcr0: Option<u64>) -> Option<crate::cpuid::Snapshot> {
    // The CPUID driver executes on the named CPU without changing caller affinity.
    // XCR0 is the calling thread's OS-managed extended-state configuration; all
    // remote CPUs must additionally report OSXSAVE before that state can qualify.
    let file = std::fs::File::open(format!("/dev/cpu/{cpu}/cpuid")).ok()?;
    Some(crate::cpuid::Snapshot {
        basic: remote_registers(&file, 0)?,
        signature: remote_registers(&file, 1)?,
        structured: remote_registers(&file, 7)?,
        extended: remote_registers(&file, 0x8000_0000)?,
        extended_features: remote_registers(&file, 0x8000_0001)?,
        xcr0,
    })
}

fn isa_homogeneous() -> Observed<bool> {
    #[cfg(target_arch = "x86_64")]
    {
        let Some(sources) = execution_domain() else {
            return Observed::Unknown;
        };
        let Some(first) = sources.first() else {
            return Observed::Unknown;
        };
        Observed::Known(sources.iter().all(|src| crate::cpuid::same_isa(first, src)))
    }
    #[cfg(target_arch = "aarch64")]
    {
        if crate::hwcap::neon_supported() {
            Observed::Known(true)
        } else {
            Observed::Unknown
        }
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        Observed::Unsupported
    }
}

fn read_number(path: impl AsRef<std::path::Path>) -> Option<u64> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn cache_lines(cpus: &[u32]) -> Option<Vec<CacheLine>> {
    let mut lines = Vec::new();
    for cpu in cpus {
        let entries = std::fs::read_dir(format!("/sys/devices/system/cpu/cpu{cpu}/cache")).ok()?;
        let start = lines.len();
        for entry in entries {
            let entry = entry.ok()?;
            if !entry.file_name().to_str()?.starts_with("index") {
                continue;
            }
            let path = entry.path();
            let level = u8::try_from(read_number(path.join("level"))?).ok()?;
            let bytes = u32::try_from(read_number(path.join("coherency_line_size"))?).ok()?;
            if bytes == 0 {
                return None;
            }
            lines.push(CacheLine {
                cpu: *cpu,
                level,
                bytes,
            });
        }
        if lines.len() == start {
            return None;
        }
    }
    lines.sort_by_key(|line| (line.cpu, line.level, line.bytes));
    Some(lines)
}

fn parse_cpu_list(text: &str) -> Option<Vec<u32>> {
    let mut result = Vec::new();
    for part in text.trim().split(',') {
        let mut ends = part.split('-');
        let first: u32 = ends.next()?.parse().ok()?;
        let last = match ends.next() {
            Some(last) => last.parse().ok()?,
            None => first,
        };
        if first > last || last > 65535 || ends.next().is_some() {
            return None;
        }
        result.extend(first..=last);
    }
    result.sort_unstable();
    result.dedup();
    Some(result)
}

fn smt_siblings(cpus: &[u32]) -> Option<Vec<SmtSiblings>> {
    cpus.iter()
        .map(|cpu| {
            let text = std::fs::read_to_string(format!(
                "/sys/devices/system/cpu/cpu{cpu}/topology/thread_siblings_list"
            ))
            .ok()?;
            Some(SmtSiblings {
                cpu: *cpu,
                siblings: parse_cpu_list(&text)?,
            })
        })
        .collect()
}

fn huge_pages() -> Option<HugePageConfig> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let value = |key: &str| -> Option<u64> {
        text.lines().find_map(|line| {
            let rest = line.strip_prefix(key)?;
            rest.split_whitespace().next()?.parse().ok()
        })
    };
    Some(HugePageConfig {
        size_kib: value("Hugepagesize:")?,
        total: value("HugePages_Total:")?,
        free: value("HugePages_Free:")?,
    })
}

fn invariant_tsc(cpus: Option<&[u32]>) -> Observed<bool> {
    #[cfg(target_arch = "x86_64")]
    {
        let Some(cpus) = cpus else {
            return Observed::Unknown;
        };
        if cpus.len() == 1 {
            let src = HardwareCpuid;
            return Observed::Known(
                crate::cpuid::available(&src, 0x8000_0007)
                    && src.cpuid(0x8000_0007, 0).edx & (1 << 8) != 0,
            );
        }
        #[cfg(target_os = "linux")]
        {
            let mut all = true;
            for cpu in cpus {
                let Ok(file) = std::fs::File::open(format!("/dev/cpu/{cpu}/cpuid")) else {
                    return Observed::Unknown;
                };
                let Some(max) = remote_registers(&file, 0x8000_0000) else {
                    return Observed::Unknown;
                };
                if max.eax < 0x8000_0007 {
                    all = false;
                    continue;
                }
                let Some(r) = remote_registers(&file, 0x8000_0007) else {
                    return Observed::Unknown;
                };
                all &= r.edx & (1 << 8) != 0;
            }
            Observed::Known(all)
        }
        #[cfg(not(target_os = "linux"))]
        {
            Observed::Unknown
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = cpus;
        Observed::Unsupported
    }
}
