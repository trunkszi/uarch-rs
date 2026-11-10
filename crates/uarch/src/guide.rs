//! Microarchitecture coding constitution for agents.
//!
//! Downstream `AGENTS.md` files point here. Rules the compiler can see are
//! pinned by doctests in this module, so `cargo test --doc -p uarch` fails
//! when the constitution and the macros drift apart.
//!
//! # Top-down Microarchitecture Analysis
//!
//! Design every hot loop against one primary bottleneck. A transform that
//! helps a different quadrant often makes the real one worse.
//!
//! ## Memory Bound
//!
//! The core is stalled on data that is not in the right cache.
//!
//! - Pack related hot values densely. Separate independently written values
//!   with [`Isolated`](crate::layout::Isolated) (128-byte alignment).
//! - Stream sequential, cache-line-dense records. A dependent pointer chase
//!   burns the memory pipes and the ROB.
//! - Prefetch a line you are about to read. Do not write-prefetch
//!   (`prefetchw` / [`PrefetchWrite`](crate::arch::PrefetchWrite), `prfm pstl1keep`)
//!   a line another core may read or write. The request takes the line
//!   Exclusive, forces an RFO, and is still only a hint.
//! - `#[cache_line]` bodies are exact multiples of 64 bytes. A short struct
//!   is rejected so the next field cannot silently share the line.
//!
//! ```
//! use uarch::cache_line;
//!
//! #[cache_line]
//! struct Line {
//!     raw: [u8; 64],
//! }
//!
//! let size = core::mem::size_of::<Line>();
//! let align = core::mem::align_of::<Line>();
//! assert_eq!(size, 64);
//! assert_eq!(align, 64);
//! ```
//!
//! ```compile_fail
//! use uarch::cache_line;
//!
//! #[cache_line]
//! struct ShortLine {
//!     body: [u8; 56],
//! }
//! ```
//!
//! ## Bad Speculation
//!
//! The frontend fetched the wrong path. See [Predication](#predication).
//!
//! - Data-dependent branches near 50/50 use [`predicate`](crate::flow::predicate) and
//!   `select` / `select_register`. `select` is `core::hint::select_unpredictable`:
//!   a compiler hint, not a promise of `cmov` / `csel`.
//! - Branches that are taken or not-taken well above 99% stay as `if`.
//!   Put [`cold_path`](crate::cold_path) on the rare arm, or mark the rare
//!   function with `#[cold_sink]` (`#[cold] #[inline(never)]`).
//! - Do not rewrite a predictable branch as `select`. `cmov` waits for both
//!   arms and lengthens the critical path.
//!
//! ## Core Bound
//!
//! The backend ports are full and the instruction stream is already in cache.
//!
//! - Keep independent accumulators. [`reduce_lanes`](crate::flow::reduce_lanes)
//!   starts each lane at the monoid identity and folds `init` once at the end.
//! - Integer division of a runtime divisor is a long port occupancy.
//!   [`FastDivisorU64`](crate::arithmetic::FastDivisorU64) and
//!   [`FastDivisorU32`](crate::arithmetic::FastDivisorU32) replace it with a multiply.
//! - [`PowerOfTwo`](crate::arithmetic::PowerOfTwo) is an `and` with a compile-time mask.
//! - `wrapping_add_top_field` is only for a field whose high bit is 63, so the add cannot
//!   carry into a neighbor. Any other field uses `wrapping_add_field`.
//!
//! ## Front-End Bound
//!
//! Fetch, decode, or the uop cache is the limit.
//!
//! - Mark the hot body `#[hot_inline]` (`#[inline(always)]`) and keep cold
//!   failure paths in `#[cold_sink]` so they land in `.text.unlikely.*`.
//! - `#[multiversion]` clones one generic `arch: A` body per tier and stamps
//!   target `#[cfg(target_arch = "...")]` plus `#[target_feature]`. Do not
//!   add a second `#[inline(always)]` on those clones, and do not hand-write
//!   `#[target_feature]` on production functions.
//! - Issue one `Tier::dispatch` at the edge of the loop. Do not pay a
//!   function-pointer or `dyn` dispatch per record.
//! - Pack hot flag tests into bits `0..=30` so x86 can encode the mask as
//!   `imm32`. Counters that use `wrapping_add_top_field` sit at `HI == 63`.
//!
//! # Structural search (`ast-grep`)
//!
//! Search and audit with the syntax tree, not a text grep. The command form
//! is `ast-grep run -p '<pattern>' -l rust`. Invoke `ast-grep` directly:
//! on many Linux distributions `/usr/bin/sg` belongs to `shadow-utils`.
//!
//! ```text
//! ast-grep run -p 'unwrap()' -l rust
//! ast-grep run -p 'expect($$$ARGS)' -l rust
//! ast-grep run -p '#[target_feature($$$ARGS)]' -l rust
//! ast-grep run -p '$META &= $FLAG' -l rust
//! ast-grep run -p 'prefetchw' -l rust
//! ast-grep run -p 'PrefetchWrite' -l rust
//! ast-grep run -p 'core::arch::asm!($$$BODY)' -l rust
//! ast-grep run -p 'std::arch::asm!($$$BODY)' -l rust
//! ```
//!
//! Reject production hits of:
//!
//! - `.unwrap()` / `.expect()` on any path that can run per record.
//! - A bare `#[target_feature]` that did not come from `#[multiversion]`.
//! - `meta &= FLAG` or `meta &= pattern`. Clearing a flag is
//!   `meta &= !FLAG` (`FlagClear`). `&=` on a positive flag is not implemented.
//! - `prefetchw` / `PrefetchWrite` aimed at a line that is not private to this core.
//! - Inline `asm!` outside `uarch-arch` / `uarch-core` tier backends.
//!
//! Bit tests and updates go through `pattern!` / `patch!`, which expand to a
//! `const` chain on [`BitPattern`](crate::bits::BitPattern) /
//! [`BitPatch`](crate::bits::BitPatch). A runtime `&` of flags is the
//! fallback and still panics on conflict in every profile.
//!
//! # Predication
//!
//! Unpredictable, data-dependent choice:
//!
//! ```
//! use uarch::flow::predicate;
//!
//! let mixed = [true, false, true, true];
//! let mut acc = 0u64;
//! for (i, flag) in mixed.iter().enumerate() {
//!     let arm = predicate(*flag).select(i as u64, 0);
//!     acc = acc.wrapping_add(arm);
//! }
//! assert_eq!(acc, 5);
//! ```
//!
//! Predictable rejection stays a branch. `cold_path` only marks the rare arm:
//!
//! ```
//! use uarch::cold_path;
//!
//! fn accept(ready: bool, value: u64) -> u64 {
//!     if !ready {
//!         cold_path();
//!         return 0;
//!     }
//!     value
//! }
//!
//! assert_eq!(accept(true, 9), 9);
//! assert_eq!(accept(false, 9), 0);
//! ```
//!
//! # Bit layout
//!
//! Hot query flags occupy bits `0..=30`. A counter that needs `wrapping_add_top_field`
//! occupies a field with `HI == 63`. `pattern!` conflicts and out-of-range
//! constants fail in `const` evaluation. `wrapping_add_top_field` does not exist for any
//! other field.
//!
//! ```
//! use uarch::prelude::*;
//!
//! bitword! {
//!     pub struct TaskMeta(u64) {
//!         [0] pub READY,
//!         [1] pub ACTIVE,
//!         [8..=11] pub STATE: u8,
//!         [32..=63] pub SEQ: u32,
//!     }
//! }
//!
//! fn main() {
//!     let meta = TaskMeta::default();
//!     let idle = pattern!(TaskMeta { READY & !ACTIVE & STATE == 0 });
//!     assert!(!meta.matches(idle));
//!     let armed = meta.patch(patch!(TaskMeta { READY: true, STATE: 1, SEQ: 4 }));
//!     assert!(armed.matches(pattern!(TaskMeta { READY & STATE == 1 })));
//!     let next = armed.wrapping_add_top_field(TaskMeta::SEQ, 1u64);
//!     assert!(next.matches(pattern!(TaskMeta { SEQ == 5 & READY })));
//! }
//! ```
//!
//! Conflicting flags are rejected while the pattern is built:
//!
//! ```compile_fail
//! use uarch::prelude::*;
//!
//! bitword! {
//!     pub struct Meta(u64) {
//!         [0] pub READY,
//!     }
//! }
//!
//! fn main() {
//!     const BAD: uarch::bits::BitPattern<Meta> = pattern!(Meta { READY & !READY });
//!     let _ = BAD;
//! }
//! ```
//!
//! `&=` does not accept a positive flag:
//!
//! ```compile_fail
//! use uarch::prelude::*;
//!
//! bitword! {
//!     pub struct Meta(u64) {
//!         [0] pub READY,
//!     }
//! }
//!
//! fn main() {
//!     let mut meta = Meta::default();
//!     meta &= Meta::READY;
//! }
//! ```
//!
//! `wrapping_add_top_field` refuses a field that does not end at bit 63:
//!
//! ```compile_fail
//! use uarch::prelude::*;
//!
//! bitword! {
//!     pub struct Meta(u64) {
//!         [8..=11] pub STATE: u8,
//!         [32..=63] pub SEQ: u32,
//!     }
//! }
//!
//! fn main() {
//!     let meta = Meta::default();
//!     let _ = meta.wrapping_add_top_field(Meta::STATE, 1u64);
//! }
//! ```
//!
//! # Register passing
//!
//! `#[register_pass]` is `#[repr(C)]` plus a proof that every field is a
//! [`RegisterLeaf`](crate::layout::RegisterLeaf) and that the aggregate is either at
//! most 8 bytes or a 9..=16 byte pair of exactly two leaves. Three leaves
//! that happen to add up to 16 bytes still go through memory (`sret`).
//! The caller writes `Clone + Copy`; the macro checks `Copy` instead of
//! deriving it again. `#[register_pass(c_abi)]` is the `extern "C"` rule
//! (`size <= 16` and `align <= 8`) and is not the Rust ABI rule.
//!
//! ```
//! use uarch::register_pass;
//!
//! #[register_pass]
//! #[derive(Clone, Copy)]
//! struct Pair {
//!     left: u64,
//!     right: i64,
//! }
//!
//! let size = core::mem::size_of::<Pair>();
//! assert_eq!(size, 16);
//! ```
//!
//! Padding holes are rejected:
//!
//! ```compile_fail
//! use uarch::register_pass;
//!
//! #[register_pass]
//! #[derive(Clone, Copy)]
//! struct PadGap {
//!     head: u8,
//!     tail: u64,
//! }
//! ```
//!
//! Three leaves in 16 bytes are rejected:
//!
//! ```compile_fail
//! use uarch::register_pass;
//!
//! #[register_pass]
//! #[derive(Clone, Copy)]
//! struct ThreeLeaf {
//!     a: u64,
//!     b: u32,
//!     c: u32,
//! }
//! ```
//!
//! # What this crate does not do
//!
//! The facade re-exports. It does not probe CPUs, invent tokens, or run a
//! runtime. `crate::detect` observes the host and is the only safe mint
//! for arch tokens. `classify` returns a [`TierId`](crate::tiers::TierId)
//! and cannot be turned into a token.
//!
//! Build the hot path so a later `cargo xtask asm-check` / `mca-gate` can
//! see straight-line code: no per-record allocation, no `dyn`, no syscall,
//! no lock.
//!
//! # Execution-domain setup
//!
//! Pin each worker thread before calling `uarch::detect::detect(...)`.
//! `taskset -c 2 program` pins a process at launch; `sched_setaffinity` can
//! pin an individual worker thread before it obtains an execution capability.
//! Keep the allowed CPU set unchanged for the lifetime of that capability.
//! An unpinned multi-CPU affinity mask without access to `/dev/cpu/N/cpuid`
//! cannot establish a complete execution-domain snapshot. `CommonSubset`
//! conservatively returns `Scalar`; `RequireHomogeneous` returns an error.
//! Hypervisors make PEXT/PDEP cost unknown. For an otherwise v3-capable domain,
//! `UnknownPdep::Conservative` selects `X86V3SlowPdep`.
//!
//! ```
//! #[cfg(feature = "detect")]
//! {
//! use uarch::detect::{DetectPolicy, ExecutionDomainPolicy, UnknownPdep};
//! let policy = DetectPolicy {
//!     domain: ExecutionDomainPolicy::CommonSubset,
//!     unknown_pdep: UnknownPdep::Conservative,
//!     ..DetectPolicy::default()
//! };
//! // Affinity is configured by the caller before capability issuance.
//! let capability = uarch::detect::detect(policy);
//! assert!(capability.is_ok());
//! }
//! ```
//!
//! Construct `BitFlag` and `BitField` descriptors exclusively through
//! `bitword!`. Its declarations check storage width, value width, and overlap.
//! `wrapping_add_top_field` requires `HI == 63`, the MSB of a 64-bit backing word.
