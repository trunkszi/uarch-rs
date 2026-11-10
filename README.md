# uarch-rs

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-2024_Edition_(MSRV_1.95)-orange.svg)](https://www.rust-lang.org/)
[![no_std](https://img.shields.io/badge/Core_%26_Arch-no__std-blue.svg)](crates/uarch-core)
[![ISA](https://img.shields.io/badge/ISA-x86__64_%26_AArch64-green.svg)](crates/uarch-arch)
[![Verification](https://img.shields.io/badge/Verification-LLVM_IR_%2F_ASM_%2F_MCA_%2F_Miri-purple.svg)](xtask)

**Turn microarchitectural intent into proven Rust layouts, explicit dataflow, audited ISA dispatch, and measurable hot-loop improvements.**

[English](README.md) | [Chinese (zh-CN)](README.zh-CN.md)

</div>

---

## The problem it solves

Safe Rust can compile into excellent machine code. Its type system alone does not promise that a small record travels in registers, that independent writers occupy separate cache lines, or that a generic call inherits the ISA features of its caller.

Tight loops can lose work across the four top-level Top-Down Microarchitecture Analysis (TMA) categories:

- **Retiring:** repeated descriptor interpretation, shifts and scalar arithmetic can spend instructions on work that could be fused or precomputed.
- **Bad Speculation:** unpredictable branches waste work on paths that will be discarded.
- **Frontend Bound:** code size, decode pressure and misplaced cold paths starve execution of useful instructions.
- **Backend Bound:** long arithmetic dependencies, division, cache misses and pointer chasing stall execution resources.

Hand-written intrinsics and scattered `unsafe` calls move those assumptions into every caller. **uarch-rs puts them behind typed primitives and explicit proof boundaries:** const-checked layouts and descriptors, branchless dataflow, strength-reduced arithmetic, execution-domain-aware SIMD dispatch, and reproducible LLVM IR / ASM / `llvm-mca` / Miri gates. Run those gates in CI to keep the intended compiler behavior observable.

## What you get

| Outcome | How uarch-rs delivers it |
| :--- | :--- |
| **Layout contracts fail early** | `#[cache_line]`, `#[register_pass]`, `#[isolated]`, `NoPadding` and `RegisterLeaf` check representation at compile time. Concrete IR gates catch calling-convention memory fallback. |
| **Compact flag queries and updates** | `bitword!`, `pattern!` and `patch!` bind descriptors to one storage word and reject conflicting or overflowing constants. |
| **Controlled loop dataflow** | `predicate` / `Predicate`, `select_register`, independent reduction lanes and bounded unrolling make intended dependencies explicit. |
| **Cheaper invariant arithmetic** | `FastDivisorU64`, `FastDivisorU32` and `PowerOfTwo` move divisor/mask work out of hot paths. |
| **ISA use backed by real evidence** | Sealed arch tokens, pure classification, execution-domain detection, and generated target-side multiversion clones separate observation from authority. |
| **Reviewable performance evidence** | Concrete symbol gates, modeled throughput, portable Miri paths, and output-equivalent allocation-checked benchmarks expose regressions and measurement limits. |

## Typical use cases

uarch-rs contains no application-specific logic. It supplies building blocks for:

- Packed status words and state-machine predicates in parsers, schedulers and storage engines.
- Fixed-capacity indexed tables with bounded masking and explicit prefetch hints.
- Batch reductions and scan kernels with independent accumulators.
- CPU-specific gather/scatter and four-lane scan kernels selected outside the per-record loop.
- Layout/ABI contracts for small hot records and independently written counters.
- Compiler-regression checks for latency-sensitive systems, including simulation, telemetry and trading software.

### When uarch-rs is not the right tool

- **Unmeasured application code:** first identify the limiting loop and its workload; a predictable branch can outperform predication.
- **A constant-time security guarantee:** both selection arms evaluate, and surrounding loads/control flow still matter.
- **An allocator, transport or shared-memory protocol:** `Pool` borrows initialized storage; this library does not own a runtime, database or IPC stack.
- **Universal ABI or throughput guarantees:** concrete signatures, targets, flags and CPU models determine the actual result. Layout macros do not certify every call site or machine.

---

## Choosing a primitive

| Bottleneck or need | Primitive | Contract to keep |
| :--- | :--- | :--- |
| Backend / false sharing and hot record layout | `#[cache_line]`, `#[isolated]`, `Isolated<T>` | Cache-line bodies fill exact configured sizes; isolated storage uses 128-byte alignment. |
| Small values crossing a hot call boundary | `#[register_pass]` | Scalar-leaf/no-padding checks plus concrete LLVM signatures; register pressure still matters. |
| Retiring / repeated flag interpretation | `bitword!`, `pattern!`, `patch!` | Const descriptors belong to their backing word; conflicts and overflow are rejected. |
| Bad speculation / independent work | `predicate`, `Predicate::select_register`, `reduce_lanes`, `reduce_four_way!` | Both arms evaluate; reductions require the documented monoid contract. |
| Backend / invariant integer division | `FastDivisorU64::div_rem`, `FastDivisorU32`, `PowerOfTwo` | Divisors are nonzero; precompute outside the loop. |
| Backend / indexed working sets | `Pool`, `PoolIndex`, `Pool::prefetch_read::<L1>` | Borrowed initialized power-of-two storage; slot zero is a sentinel. Prefetch is a hint. |
| ISA specialization | `#[multiversion]`, `uarch::detect::detect` | Audit the execution domain, preserve it for token lifetime, dispatch at a batch boundary. |
| Frontend / hot and cold placement | `#[hot_inline]`, `#[cold_sink]`, `cold_path` | Inlining can increase code size; `cold_sink` accepts Rust ABI functions. |

---

## Quick start

### Add the dependency

```toml
[dependencies]
uarch = { git = "https://github.com/trunkszi/uarch-rs.git", branch = "main" }
```

The default feature set enables `std` and `detect` (host CPUID and Linux affinity auditing). For `no_std` crates, disable default features:

```toml
[dependencies]
uarch = { git = "https://github.com/trunkszi/uarch-rs.git", branch = "main", default-features = false }
```

### Layout and bit algebra

```rust
use uarch::prelude::*;

bitword! {
    pub struct Meta(u64) {
        [0] pub READY,
        [1] pub BUSY,
        [8..=11] pub STATE: u8,
    }
}

#[register_pass]
#[derive(Clone, Copy)]
struct Pair {
    left: u64,
    right: u64,
}

#[cache_line]
struct Line {
    bytes: [u8; 64],
}

fn main() {
    let meta = Meta::default().patch(patch!(Meta { READY: true, STATE: 3 }));
    assert!(meta.matches(pattern!(Meta { READY & !BUSY & STATE == 3 })));
    let pair = Pair { left: 1, right: 2 };
    assert_eq!(pair.left + pair.right, 3);
    let line = Line { bytes: [0; 64] };
    assert_eq!(line.bytes[0], 0);
    assert_eq!(core::mem::size_of::<Pair>(), 16);
    assert_eq!(core::mem::size_of::<Line>(), 64);
    assert_eq!(core::mem::align_of::<Line>(), 64);
}
```

`#[cache_line]` rejects an underfilled 56-byte payload. `#[register_pass]` checks the leaf structure as well as size; it does not certify arbitrary register-pressure call signatures.

### Arithmetic, indexed storage and predication

```rust
use core::num::NonZeroU64;
use uarch::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let divisor = NonZeroU64::new(7).ok_or("zero divisor")?;
    let fast = FastDivisorU64::new(divisor);
    assert_eq!(fast.div_rem(100), (14, 2));
    assert_eq!(PowerOfTwo::<3>::wrap_usize(11), 3);

    let mut storage = [0u64; 8];
    let mut pool = Pool::<u64, 3>::new(&mut storage)?;
    let index = PoolIndex::new(3).ok_or("zero is reserved")?;
    *pool.get_mut(index).ok_or("index out of bounds")? = 21;
    pool.prefetch_read::<uarch::mem::L1>(index);
    let value = *pool.get_masked(index);
    assert_eq!(predicate(true).select_register(value, 0), 21);

    let sum = reduce_lanes::<4, _, _>(
        &[1u64, 2, 3, 4], 10, 0,
        |acc, item| acc.wrapping_add(*item), u64::wrapping_add,
    );
    assert_eq!(sum, 20);
    Ok(())
}
```

`predicate(flag)`, `Predicate::new(flag)`, and `Predicate::from(flag)` construct the same normalized predicate.

`select_register` evaluates both arms, so it cannot guard an invalid access. `get_masked` can deliberately map an oversized index onto the sentinel; use `get` when checked physical indexing is intended. `reduce_lanes` uses identity for each lane and combines init once.

### Detect once, dispatch at the boundary

```rust
use uarch::{ArchVisitor, prelude::*};

#[multiversion]
fn gather<A: Arch>(arch: A, value: u64, mask: u64) -> u64 {
    arch.bit_gather(value, mask)
}

struct Gather;
impl ArchVisitor for Gather {
    type Output = u64;
    fn visit<A: Arch>(self, arch: A) -> u64 {
        gather(arch, 0xa5, 0xf0)
    }
}

fn main() -> Result<(), uarch::detect::DetectError> {
    let tier = uarch::detect::detect(DetectPolicy::default())?;
    assert_eq!(tier.dispatch(Gather), 10);
    Ok(())
}
```

Configure worker affinity before detection and preserve the audited CPU domain for all threads receiving a token. On Linux, multi-CPU x86 audits need readable `/dev/cpu/N/cpuid`; default policy returns Scalar if evidence is unavailable, while `RequireHomogeneous` returns an error. The detector does not change affinity. Default `Max256` avoids automatic AVX-512 selection, and conservative unknown-cost policy avoids assuming fast PEXT/PDEP in virtualized environments.

### Run the Bad-vs-Good suite

```bash
cargo xtask bench
```

The suite checks output parity, batches short operations above timer resolution, compares identical evolving inputs, and reports P50/P90/P99/P99.9 with hot-region allocation assertions. Pair labels describe workload patterns, not measured TMA attribution; compare equal work within a pair. Results depend on the host and build.

---

`FastDivisorU32` and `FastDivisorU64` share const `new`, `divisor`, `is_power_of_two`, `div`, `rem`, `div_rem` and `is_multiple_of`. `PowerOfTwo` offers explicit-width division, remainders and checked alignment arithmetic. `Bitmap64` supports forward/reverse set-bit traversal, set algebra and const updates.

`CacheAligned<T>` and `Isolated<T>` align storage to at least 64 and 128 bytes with the same value access and standard traits. They may contain padding and do not imply `NoPadding`. `#[cache_line(lines = N)]` proves exactly N full 64-byte lines, including 192-byte and 320-byte records. `#[isolated]` selects 128-byte alignment; `#[isolated(full)]` also proves exact 128-byte occupancy. `assert_layout!` locks size, alignment and optional offsets.

`likely` / `unlikely` preserve boolean results and hint cold edges; they are not timing guarantees. Reference prefetch uses type markers: `uarch::arch::prefetch_read_ref::<_, uarch::mem::L1>(&value)`; write prefetch additionally requires `PrefetchWrite`. `TierId::compiled()` describes target features without granting execution authority; `Tier::id()` identifies a held capability.

## Auditing codegen and soundness: cargo xtask

```bash
cargo xtask lint
cargo xtask msrv
cargo xtask ui
cargo xtask abi-check
cargo xtask ir-check
cargo xtask asm-check
cargo xtask mca-gate
cargo xtask size-report
cargo xtask verify
cargo xtask pmu-gate --probe flow_select_register_u64 --iterations 100000
cargo xtask pmu-gate --executable "$BENCH_BINARY" --operations 1
cargo xtask miri
cargo xtask bench
```

| Gate | Evidence |
| :--- | :--- |
| `lint` | Format, workspace Clippy, manifest/mirror/hygiene checks, locked release scenarios |
| `msrv` | Rust 1.95 workspace checks and portable semantics |
| `ui` | All-target build and macro pass/fail snapshots, including real facade proofs |
| `abi-check` | Concrete LLVM signatures and a register-pressure negative control |
| `ir-check` | Concrete LLVM definitions and invariant-division lowering |
| `asm-check` | Full-symbol opcode budgets, required/forbidden instructions and cold sections across five x86 CPU configurations |
| `mca-gate` | Compiler-assembly regions, marker parity and modeled throughput budgets |
| `size-report` | Actual native code size for each audited symbol |
| `verify` | Aggregate feature checks, tests, MSRV, ABI/IR/ASM/MCA and symbol sizes |
| `pmu-gate` | Instructions and branch misses for named probe iterations or an explicit executable/work denominator |
| `miri` | Core/arch portable paths under pinned Miri defaults and Tree Borrows |
| `bench` | Workspace latency, output-equivalence and allocation checks |

Set `BENCH_BINARY` to the actual path printed by `cargo bench -p uarch --bench main --no-run --locked`. In the executable command above, one operation is the whole benchmark process, including startup/harness overhead. Named-probe counts include loop, runner, and startup overhead. Hardware permissions and events are checked; unavailable measurements are `NotRun`/`Unsupported`, never an invented pass. Use `cargo xtask <gate> --help` for the exact interface.

`CARGO_TARGET_DIR=/tmp/uarch-check-target cargo xtask lint` keeps artifacts outside the checkout. Codegen audit builds disable LTO locally to emit native ELF objects directly; normal release/bench builds retain Thin LTO.

## Platforms and toolchains

| Area | Support / requirement |
| :--- | :--- |
| Rust | Edition 2024, MSRV 1.95.0; gates use the active `nightly` toolchain from `rust-toolchain.toml` |
| Primitive libraries | `uarch-tiers`, `uarch-core`, `uarch-arch` are `no_std` |
| ISA paths | x86_64 and AArch64 kernels; scalar/portable fallbacks where implemented |
| Host detection | Linux execution-domain evidence; permissions and CPU/OS state can force a conservative fallback |
| Codegen gates | Artifacts from the active compiler; LLVM tools must match its LLVM major version; current CPU/throughput gate matrix is x86_64 |
| Miri | Active `nightly` with `rust-src` and `miri`; `UARCH_MIRI_TOOLCHAIN` override |
| Hardware counters | Linux `perf` with appropriate permissions; measured separately from modeled MCA budgets |

Install `rustfmt`, `clippy`, and `rust-src` on active `nightly` so UI diagnostics match CI; add `miri` for Miri gates. Install its optional llvm-tools-preview component for bundled objdump; system objdump is a fallback. `llvm-mca` must match the active compiler's LLVM major version, and `perf` is an external tool. Native objdump prefers the active sysroot binary. AArch64 source support does not substitute for an AArch64 hardware validation run.

## Repository layout

```text
crates/uarch-tiers/   Tier IDs and shared feature lists; no_std, no dependencies
crates/uarch-core/    Layout, bits, flow, arithmetic and pools; no_std
crates/uarch-macros/  Attribute macros and const-checked bit DSL
crates/uarch-arch/    Sealed capability tokens and ISA kernels; no_std
crates/uarch-detect/  Host/execution-domain evidence and conservative policies
crates/uarch/         Public facade, prelude and coding guide
crates/uarch-probes/  Concrete symbols and typed codegen contracts
xtask/                Semantic, compiler, modeling and hardware gates
```

## Contributing

For ordinary delivery checks:

```bash
cargo xtask lint
cargo test --workspace --all-targets --locked
cargo test --workspace --all-targets --release --locked
cargo test --workspace --doc --locked
```

Run the additional codegen, Miri and hardware gates appropriate to the changed contract.

Distributed under the MIT license; see [LICENSE](LICENSE).

### O3 model measurements and MCA matrices

```sh
cargo xtask mca-gate --cpu znver4,znver5,sapphirerapids
UARCH_MCA_CPU=znver4 cargo xtask mca-gate
cargo xtask gem5-gate --cpu o3-w6-rob320
UARCH_GEM5_BIN=/path/to/gem5.opt cargo xtask gem5-gate --cpu zen4-o3,zen5-o3
python3 scripts/gem5/o3_sim.py --profile neoverse-v2 --dump-profile-json
```

MCA checks 29 concrete probes, using baseline x86-64, x86-64-v3, or x86-64-v4 compilation as appropriate. The default x86 model matrix is `znver4,znver5,sapphirerapids`; AArch64 selects `neoverse-v2` when architecture-specific probe budgets are available. Overrides accept comma-separated LLVM CPU models. Empty elements and malformed names fail. Each probe's reference model uses its declared bound; other models use 1.25 times that bound. The assembly CPU and modeled CPU are reported separately. This does not execute another CPU's instructions on the host.

`gem5-gate` builds an optimized guest path and verifies all seven portable probes natively at N=64 before resolving `UARCH_GEM5_BIN` or searching PATH for `gem5.fast`/`gem5.opt`. An explicit invalid path fails; an absent simulator reports `NotRun` with nonzero exit status. No simulator is downloaded. The freestanding static ELF guest has no libc, dynamic interpreter or CLI runtime. It uses raw write/exit_group syscalls and monomorphic target/checksum loops; the host oracle stays outside the guest. Both argument forms, with or without `__gem5-probe`, are accepted.

Independent simulations run in parallel and results are evaluated in deterministic model/probe order. Each model/probe uses three separate executions with fixed-width iteration arguments `0000000512`, `0000002560`, `0000004608`. Only additive raw counters are subtracted; adjacent cycles/op slopes must agree within 5%. The host independently verifies every checksum. Both host launch and guest environment disable glibc rseq using `GLIBC_TUNABLES=glibc.pthread.rseq=0`. A 120-second process bound and simulated tick limit reject incomplete runs.

Profiles `o3-w6-rob320`, `o3-w8-rob448`, `o3-w6-rob512`, `o3-a64-w6-rob320` expose widths, queues, functional units, predictor and cache settings via `--dump-profile-json`. Their `zen4-o3`/`znver4`, `zen5-o3`/`znver5`, `sapphirerapids-o3`/`sapphirerapids`, `neoverse-v2-o3`/`neoverse-v2` aliases are inspired-by labels, not calibrated replicas. The guest ELF ISA must match its profile. No Atomic/O3 switching or m5 magic operations are injected into shared probes.

The gate reports `o3-rename-slot` model attribution, not hardware TMA. It uses committed operations, rename counts, idle and squash cycles; resource-full events remain separate per-operation pressure metrics. Missing rename inputs, malformed/ambiguous stats, negative deltas, non-stationarity or a raw slot sum above 105% fail. Legacy all-commit fields are preferred over filtered compatibility counters. Branchless probes require at most 0.005 branch mispredictions/op; all probes require zero memory-order violations. Harness-inclusive ceilings are 32 cycles/op for the six baseline probes and 256 for scalar gather. A positive finite `UARCH_GEM5_MAX_CYCLES_PER_OP` overrides these ceilings.

The mandatory set is `bitword_low_matches`, `bitword_high_matches`, `flow_select_register_u64`, `flow_select_u64`, `fast_divisor_u64_div`, `fast_divisor_u32_rem`; scalar gather is also measured. Extended BMI/VEX/SIMD probes are explicitly reported as `Unsupported`/not admitted in this Rust runner and are not silently replaced with scalar implementations. Mock simulator tests establish parser/orchestration rejection behavior, not real gem5 decoder support, timing accuracy or hardware performance.

## License

Licensed under MIT.
