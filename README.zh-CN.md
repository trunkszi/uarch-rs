# uarch-rs

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-2024_Edition_(MSRV_1.95)-orange.svg)](https://www.rust-lang.org/)
[![no_std](https://img.shields.io/badge/Core_%26_Arch-no__std-blue.svg)](crates/uarch-core)
[![ISA](https://img.shields.io/badge/ISA-x86__64_%26_AArch64-green.svg)](crates/uarch-arch)
[![Verification](https://img.shields.io/badge/Verification-LLVM_IR_%2F_ASM_%2F_MCA_%2F_Miri-purple.svg)](xtask)

**把微架构意图变成可证明的 Rust 布局、显式数据流、经过审计的 ISA 分派和可实测的热循环改进。**

[English](README.md) | [简体中文](README.zh-CN.md)

</div>

---

## 它解决什么问题

安全 Rust 可以生成优秀的机器码，但类型系统本身并不保证小结构体一定通过寄存器传递、独立写入的数据一定不共享缓存行，或泛型调用自动继承调用方的 ISA 特征。

紧凑热循环仍可能在 Top-Down Microarchitecture Analysis（TMA）的四个顶层类别上浪费资源：

- **Retiring（指令退休）**：重复解释描述符、移位和标量运算，占用了本可通过融合或预计算省下的指令槽位。
- **Bad Speculation（错误推测）**：难以预测的分支让处理器执行最终会被丢弃的路径。
- **Frontend Bound（前端受限）**：代码体积、解码压力和冷热路径混排限制有效指令供给。
- **Backend Bound（后端受限）**：长算术依赖、除法、缓存缺失和指针追逐阻塞执行资源。

手写 intrinsic 和散落的 `unsafe` 调用会把这些假设分散到每个调用点。**uarch-rs 把它们收进类型化原语与明确的证明边界**：编译期布局与描述符校验、无分支数据流、算术强度削减、执行域感知的 SIMD 分派，以及可重复运行的 LLVM IR / ASM / `llvm-mca` / Miri 门禁。把这些命令接入 CI，才能持续观察预期的编译器行为。

## 你能得到什么

| 收益 | uarch-rs 如何做到 |
| :--- | :--- |
| **布局契约提前失败** | `#[cache_line]`、`#[register_pass]`、`#[isolated]`、`NoPadding` 和 `RegisterLeaf` 在编译期检查表示；具体 IR 门禁检查调用约定是否退化为内存传递。 |
| **紧凑的标志查询与更新** | `bitword!`、`pattern!`、`patch!` 将描述符绑定到存储字，并拒绝冲突或越界常量。 |
| **可控的循环数据流** | `predicate` / `Predicate`、`select_register`、独立归约通道和有界展开显式表达依赖关系。 |
| **更便宜的不变除数算术** | `FastDivisorU64`、`FastDivisorU32`、`PowerOfTwo` 把除数和掩码的准备工作移出热循环。 |
| **有真实证据支持的 ISA 使用** | 密封能力 Token、纯分类函数、执行域探测和目标端多版本克隆，将硬件观测与执行权限分开。 |
| **可审阅的性能证据** | 具体符号门禁、吞吐模型、可移植 Miri 路径，以及结果等价、带分配断言的基准，暴露回归与测量边界。 |

## 典型使用场景

uarch-rs 不包含应用专属逻辑，常用来构建：

- 解析器、调度器、存储引擎中的紧凑状态字与状态机谓词。
- 具有有界掩码寻址和显式预取提示的定长索引表。
- 使用独立累加器的批量归约与扫描内核。
- 在逐记录循环之外选择的 CPU 专属 gather/scatter 与四通道扫描。
- 小型热记录和独立写入计数器的布局、ABI 契约。
- 仿真、遥测、交易等低延迟系统中的编译器回归检查。

### 什么时候不该用 uarch-rs

- **尚未测量的应用代码**：先确定限制性能的循环和工作负载；高度可预测的分支可能胜过谓词化。
- **需要恒定时间安全保证**：选择器两侧都会求值，周围的访存与控制流仍然重要。
- **需要分配器、传输或共享内存协议**：`Pool` 借用已初始化存储，本库不拥有运行时、数据库或 IPC 栈。
- **需要跨所有签名和 CPU 的统一 ABI、吞吐保证**：实际结果由签名、目标、编译参数和 CPU 模型共同决定；布局宏不能证明所有调用点与机器。

---

## 如何选择原语

| 瓶颈或需求 | 原语 | 必须保留的契约 |
| :--- | :--- | :--- |
| 后端、伪共享、热记录布局 | `#[cache_line]`、`#[isolated]`、`Isolated<T>` | 缓存行结构体填满配置大小；隔离存储按 128 字节对齐。 |
| 热调用边界上的小值传递 | `#[register_pass]` | 标量叶子、无填充证明加具体 LLVM 签名审计；寄存器压力仍然重要。 |
| Retiring、重复解释标志 | `bitword!`、`pattern!`、`patch!` | 常量描述符绑定存储字，拒绝冲突和溢出。 |
| 错误推测、独立工作 | `predicate`、`Predicate::select_register`、`reduce_lanes`、`reduce_four_way!` | 两侧都会求值；归约必须满足文档中的幺半群契约。 |
| 后端、不变除数整数除法 | `FastDivisorU64::div_rem`、`FastDivisorU32`、`PowerOfTwo` | 除数非零，在循环外预计算。 |
| 后端、索引工作集 | `Pool`、`PoolIndex`、`Pool::prefetch_read::<L1>` | 借用已初始化的二次幂存储，零号槽是哨兵；预取只是提示。 |
| ISA 特化 | `#[multiversion]`、`uarch::detect::detect` | 审计执行域，在 Token 生命周期内保持有效，并在批次边界分派。 |
| 前端、冷热代码放置 | `#[hot_inline]`、`#[cold_sink]`、`cold_path` | 内联可能增加代码体积；`cold_sink` 接受 Rust ABI 函数。 |

---

## 快速上手

### 添加依赖

```toml
[dependencies]
uarch = { git = "https://github.com/trunkszi/uarch-rs.git", branch = "main" }
```

默认特征集启用 `std` 与 `detect`（宿主 CPUID 与 Linux 亲和性审计）。对于 `no_std` crate，可关闭默认特征：

```toml
[dependencies]
uarch = { git = "https://github.com/trunkszi/uarch-rs.git", branch = "main", default-features = false }
```

### 布局与位代数

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

`#[cache_line]` 会拒绝只有 56 字节内容的结构体。`#[register_pass]` 同时检查叶子结构和大小，但不证明任意寄存器压力下的调用签名。

### 算术、索引存储与谓词化

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

`predicate(flag)`、`Predicate::new(flag)` 和 `Predicate::from(flag)` 构造相同的归一化谓词。

`select_register` 两侧都会求值，因此不能用来保护非法访问。`get_masked` 可以有意将越界索引映射到哨兵；需要检查物理边界时使用 `get`。`reduce_lanes` 用 identity 初始化各通道，并且只合入一次 init。

### 探测一次，在边界分派

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

在探测前配置工作线程亲和性，并让所有接收 Token 的线程始终处于已审计 CPU 域内。Linux 多 CPU x86 审计需要读取 `/dev/cpu/N/cpuid` 的权限；缺少证据时默认策略返回 Scalar，`RequireHomogeneous` 则报错。探测器本身不修改亲和性。默认 `Max256` 避免自动选择 AVX-512，保守的未知成本策略也不会假定虚拟化环境中的 PEXT/PDEP 很快。

### 运行 Bad-vs-Good 基准

```bash
cargo xtask bench
```

基准校验结果等价性，对短操作批量计时以越过计时器分辨率，用相同且持续演进的输入比较两侧，并输出 P50/P90/P99/P99.9 和热区分配断言。名称描述工作负载模式，不代表实测 TMA 归因；应在每一对相同工作量内比较。结果取决于宿主和构建参数。

---

`FastDivisorU32` 和 `FastDivisorU64` 具有相同的 const 方法：`new`、`divisor`、`is_power_of_two`、`div`、`rem`、`div_rem` 和 `is_multiple_of`。`PowerOfTwo` 提供分宽度的除法、余数与溢出检查对齐运算。`Bitmap64` 支持双向置位遍历、集合运算与 const 更新。

`CacheAligned<T>` 与 `Isolated<T>` 分别至少按 64 和 128 字节对齐，提供相同的值访问与标准 trait；两者可能有填充，不能据此推导 `NoPadding`。`#[cache_line(lines = N)]` 证明恰好占满 N 个 64 字节缓存行，包括 192 和 320 字节记录。`#[isolated]` 使用 128 字节对齐，`#[isolated(full)]` 还证明恰好占满 128 字节。`assert_layout!` 锁定大小、对齐与可选字段偏移。

`likely` / `unlikely` 保持布尔结果并提供 cold 边提示，不是时间保证。引用预取使用类型标记：`uarch::arch::prefetch_read_ref::<_, uarch::mem::L1>(&value)`；写预取还需要 `PrefetchWrite` 能力。`TierId::compiled()` 只描述编译目标，不授予运行时执行权限；`Tier::id()` 返回已持有能力的标识。

## 代码生成与正确性门禁：cargo xtask

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

| 门禁 | 提供的证据 |
| :--- | :--- |
| `lint` | 格式、全工作区 Clippy、清单/镜像/卫生检查、锁定依赖的 release scenario 测试 |
| `msrv` | Rust 1.95 工作区检查与可移植语义测试 |
| `ui` | 全目标构建、宏通过/失败快照，包括真实门面的证明路径 |
| `abi-check` | 具体 LLVM 签名和寄存器压力负对照 |
| `ir-check` | 具体符号的 LLVM IR 定义与不变量除法降级检查 |
| `asm-check` | 五种 x86 CPU 配置下完整符号的指令预算、必需/禁止指令及冷段 |
| `mca-gate` | 编译器汇编区间、标记一致性、模型吞吐预算 |
| `size-report` | 原生对象中受审计符号的实际字节数 |
| `verify` | 特性矩阵、测试、MSRV、ABI/IR/ASM/MCA 与符号大小的聚合验证 |
| `pmu-gate` | 对命名探针迭代次数或显式可执行文件/工作量分母测量指令数及分支失误 |
| `miri` | 固定 Miri 默认模型与 Tree Borrows 下的 core/arch 可移植路径 |
| `bench` | 工作区延迟、输出等价性和分配检查 |

将 `BENCH_BINARY` 设为 `cargo bench -p uarch --bench main --no-run --locked` 打印的实际可执行文件路径。上面 executable 命令中的一次 operation 是整个基准进程，包括启动和 harness 开销。命名探针的计数包括循环、runner 和启动开销。硬件权限与事件可用性会被检查；不可测量时报告 `NotRun`/`Unsupported`，不会制造 PASS。准确参数见 `cargo xtask <gate> --help`。

`CARGO_TARGET_DIR=/tmp/uarch-check-target cargo xtask lint` 可将产物放到检出之外。代码生成审计仅在探针构建时关闭 LTO，以直接生成原生 ELF；常规 release/bench 构建仍使用 Thin LTO。

## 平台与工具链支持

| 范围 | 支持或要求 |
| :--- | :--- |
| Rust | Edition 2024、MSRV 1.95.0；门禁使用 `rust-toolchain.toml` 选择的活动 `nightly` 工具链 |
| 原语库 | `uarch-tiers`、`uarch-core`、`uarch-arch` 为 `no_std` |
| ISA 路径 | x86_64 与 AArch64 内核，以及已实现的标量/可移植回退 |
| 宿主探测 | Linux 执行域证据；权限、CPU/OS 状态不足时可能保守降级 |
| 代码生成门禁 | 活动编译器产物；LLVM 工具主版本必须与编译器一致；当前 CPU/吞吐门禁矩阵是 x86_64 |
| Miri | 默认活跃 `nightly`，安装 `rust-src` 和 `miri`；可通过 `UARCH_MIRI_TOOLCHAIN` 覆盖 |
| 硬件计数器 | Linux `perf` 及相应权限，与 MCA 模型预算分开测量 |

在活动 `nightly` 上安装 `rustfmt`、`clippy` 和 `rust-src`，保持 UI 诊断与 CI 一致；Miri 门禁另需安装 `miri`。可安装其可选的 llvm-tools-preview 组件以提供 objdump，也可回退到系统 objdump。`llvm-mca` 的 LLVM 主版本必须与活动编译器一致，`perf` 是外部工具；原生反汇编优先使用活动 sysroot 内的 objdump。AArch64 源码支持不能替代真实 AArch64 硬件验证。

## 仓库结构

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

## 参与贡献

常规交付检查为：

```bash
cargo xtask lint
cargo test --workspace --all-targets --locked
cargo test --workspace --all-targets --release --locked
cargo test --workspace --doc --locked
```

根据改动契约补跑相应的代码生成、Miri 与硬件门禁。

本项目以 MIT 许可证发布，详见 [LICENSE](LICENSE)。

### O3 模型测量与 MCA 矩阵

```sh
cargo xtask mca-gate --cpu znver4,znver5,sapphirerapids
UARCH_MCA_CPU=znver4 cargo xtask mca-gate
cargo xtask gem5-gate --cpu o3-w6-rob320
UARCH_GEM5_BIN=/path/to/gem5.opt cargo xtask gem5-gate --cpu zen4-o3,zen5-o3
python3 scripts/gem5/o3_sim.py --profile neoverse-v2 --dump-profile-json
```

MCA 对 29 个具体探针按需使用 x86-64、x86-64-v3 或 x86-64-v4 编译。x86 默认模型为 `znver4,znver5,sapphirerapids`；AArch64 在存在对应探针预算时选择 `neoverse-v2`。`--cpu` 优先于 `UARCH_MCA_CPU`；列表禁止空项和非法字符。参考模型使用声明预算，其他模型使用其 1.25 倍。编译目标与建模 CPU 分开报告，不在主机执行其他 CPU 的指令。

`gem5-gate` 先构建优化 guest 路径，以 N=64 验证全部七个便携探针的原生校验和，再读取 `UARCH_GEM5_BIN` 或从 PATH 查找 `gem5.fast`/`gem5.opt`。显式无效路径失败；缺少模拟器返回 `NotRun` 和非零退出码，不自动下载。静态、无 libc 的 ELF guest 使用原始 write/exit_group 系统调用，仅执行专用探针循环与顺序敏感校验和，不运行参考循环、CLI、perf 或探测器。原生 N=64 校验覆盖七个探针。

各组并行执行，并按模型和探针顺序确定性汇总。每组使用固定宽度参数 `0000000512`、`0000002560`、`0000004608` 分别运行，只对原始可加计数做差。相邻窗口 cycles/op 斜率偏差必须不超过 5%，主机独立验证每次 guest 校验和。启动环境与 guest 环境均设置 `GLIBC_TUNABLES=glibc.pthread.rseq=0`。每次运行限制为 120 秒，配置另有模拟 tick 上限。

四个参数模型为 `o3-w6-rob320`、`o3-w8-rob448`、`o3-w6-rob512`、`o3-a64-w6-rob320`。`--dump-profile-json` 展示宽度、队列、功能单元、预测器与缓存配置。`zen4-o3`/`znver4`、`zen5-o3`/`znver5`、`sapphirerapids-o3`/`sapphirerapids`、`neoverse-v2-o3`/`neoverse-v2` 仅是启发式别名，不代表校准过的真实 CPU。guest ELF 与模型 ISA 必须一致；不切换 Atomic/O3，也不向共享探针注入 m5 魔术指令。

归因模式明确标为 `o3-rename-slot`，不等同于硬件 TMA。提交操作数、rename 数和空闲/清除周期参与模型归因；ROB/IQ/LQ/SQ 满事件仅作为独立压力指标。缺失归因输入、异常或歧义统计、负差分、非稳态以及原始槽位比例和超过 105% 均失败。优先使用旧版包含全部提交操作的字段，再兼容排除 NOP 的旧字段。无分支探针要求分支预测错误不超过 0.005/op，所有探针要求内存序违规为零。六个基线探针预算为含循环开销的 32 cycles/op，标量 gather 为 256；正有限 `UARCH_GEM5_MAX_CYCLES_PER_OP` 可覆盖预算。

六个必需探针为 `bitword_low_matches`、`bitword_high_matches`、`flow_select_register_u64`、`flow_select_u64`、`fast_divisor_u64_div`、`fast_divisor_u32_rem`，另测标量 gather。扩展 BMI/VEX/SIMD 在此 Rust runner 中明确报告未准入的 `Unsupported`，不会静默替换为标量实现。假模拟器测试只证明编排和拒绝逻辑，不证明真实 gem5 指令支持、周期精度或硬件性能。

## 许可证

采用 MIT 许可证。
