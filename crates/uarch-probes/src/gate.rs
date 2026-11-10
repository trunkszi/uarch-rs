pub use uarch_core::arithmetic::{FastDivisorU32, FastDivisorU64};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    Jcc,
    Movabs,
    Call,
    Div,
    Idiv,
    Imul,
    Udivti3,
    Pext,
    Pdep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbiRule {
    Unchecked,
    Registers,
    MemoryControl,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct McaBound {
    pub cpu: &'static str,
    pub max_block_reciprocal_throughput: f64,
}

#[derive(Debug)]
pub struct Gate {
    pub symbol: &'static str,
    pub target_arch: &'static str,
    pub deny: &'static [Instruction],
    pub require: &'static [Instruction],
    pub max_instructions: Option<usize>,
    pub abi: AbiRule,
    pub section: Option<&'static str>,
    pub mca: Option<McaBound>,
}

// Bounds cover measured LLVM scheduling models; non-reference CPUs retain the
// MCA gate's 1.25 allowance. Loop bounds describe one static block, not a whole call.
pub const GATES: &[Gate] = &[
    Gate {
        symbol: "cold_sink_body",
        target_arch: "x86_64",
        deny: &[],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: Some(".text.unlikely"),
        mca: None,
    },
    Gate {
        symbol: "bitword_low_matches",
        target_arch: "x86_64",
        deny: &[Instruction::Jcc, Instruction::Movabs, Instruction::Call],
        require: &[],
        max_instructions: Some(4),
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 1.2,
        }),
    },
    Gate {
        symbol: "bitword_high_matches",
        target_arch: "x86_64",
        deny: &[Instruction::Jcc, Instruction::Call],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 2.5,
        }),
    },
    Gate {
        symbol: "fast_divisor_u64_div",
        target_arch: "x86_64",
        deny: &[Instruction::Div, Instruction::Idiv, Instruction::Udivti3],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 3.6,
        }),
    },
    Gate {
        symbol: "fast_divisor_u64_rem",
        target_arch: "x86_64",
        deny: &[
            Instruction::Div,
            Instruction::Idiv,
            Instruction::Udivti3,
            Instruction::Call,
        ],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 4.0,
        }),
    },
    Gate {
        symbol: "fast_divisor_u64_pow2_rem",
        target_arch: "x86_64",
        deny: &[
            Instruction::Div,
            Instruction::Idiv,
            Instruction::Udivti3,
            Instruction::Call,
            Instruction::Imul,
        ],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 1.2,
        }),
    },
    Gate {
        symbol: "x86_v3_slow_pdep_gather",
        target_arch: "x86_64",
        deny: &[Instruction::Imul, Instruction::Pext, Instruction::Pdep],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 2.8,
        }),
    },
    Gate {
        symbol: "x86_v3_gather",
        target_arch: "x86_64",
        deny: &[],
        require: &[Instruction::Pext],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 1.0,
        }),
    },
    Gate {
        symbol: "cold_sink_fn",
        target_arch: "x86_64",
        deny: &[],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: Some(".text.unlikely"),
        mca: None,
    },
    Gate {
        symbol: "register_small",
        target_arch: "x86_64",
        deny: &[],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Registers,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 6.3,
        }),
    },
    Gate {
        symbol: "register_c_abi",
        target_arch: "x86_64",
        deny: &[],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Registers,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 2.8,
        }),
    },
    Gate {
        symbol: "register_pair",
        target_arch: "x86_64",
        deny: &[],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Registers,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 1.6,
        }),
    },
    Gate {
        symbol: "register_pressure_control",
        target_arch: "x86_64",
        deny: &[],
        require: &[],
        max_instructions: None,
        abi: AbiRule::MemoryControl,
        section: None,
        mca: None,
    },
    Gate {
        symbol: "fast_divisor_u32_rem",
        target_arch: "x86_64",
        deny: &[Instruction::Div, Instruction::Idiv, Instruction::Udivti3],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Registers,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 4.0,
        }),
    },
    Gate {
        symbol: "flow_select_register_u64",
        target_arch: "x86_64",
        deny: &[Instruction::Jcc, Instruction::Call],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 0.8,
        }),
    },
    Gate {
        symbol: "x86_v3_slow_pdep_scatter",
        target_arch: "x86_64",
        deny: &[Instruction::Pext, Instruction::Pdep],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 2.3,
        }),
    },
    Gate {
        symbol: "flow_likely",
        target_arch: "x86_64",
        deny: &[],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: None,
    },
    Gate {
        symbol: "flow_unlikely",
        target_arch: "x86_64",
        deny: &[],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: None,
    },
    Gate {
        symbol: "uarch_arch_scalar_gather",
        target_arch: "x86_64",
        deny: &[
            Instruction::Imul,
            Instruction::Call,
            Instruction::Div,
            Instruction::Idiv,
        ],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 3.0,
        }),
    },
    Gate {
        symbol: "uarch_arch_masked_sum",
        target_arch: "x86_64",
        deny: &[Instruction::Call, Instruction::Div, Instruction::Idiv],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 3.2,
        }),
    },
    Gate {
        symbol: "uarch_arch_first_ge",
        target_arch: "x86_64",
        deny: &[Instruction::Call, Instruction::Div, Instruction::Idiv],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 2.4,
        }),
    },
    Gate {
        symbol: "uarch_arch_prefetch_write",
        target_arch: "x86_64",
        deny: &[Instruction::Call, Instruction::Div, Instruction::Idiv],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 0.7,
        }),
    },
    Gate {
        symbol: "pool_prefetch_l1",
        target_arch: "x86_64",
        deny: &[Instruction::Call, Instruction::Div, Instruction::Idiv],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 0.7,
        }),
    },
    Gate {
        symbol: "flow_select_u64",
        target_arch: "x86_64",
        deny: &[Instruction::Call, Instruction::Div, Instruction::Idiv],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 0.8,
        }),
    },
    Gate {
        symbol: "x86_v3_scatter",
        target_arch: "x86_64",
        deny: &[Instruction::Call, Instruction::Div, Instruction::Idiv],
        require: &[Instruction::Pdep],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 1.0,
        }),
    },
    Gate {
        symbol: "x86_v4_gather",
        target_arch: "x86_64",
        deny: &[Instruction::Call, Instruction::Div, Instruction::Idiv],
        require: &[Instruction::Pext],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 1.0,
        }),
    },
    Gate {
        symbol: "x86_v4_masked_sum",
        target_arch: "x86_64",
        deny: &[Instruction::Call, Instruction::Div, Instruction::Idiv],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 2.4,
        }),
    },
    Gate {
        symbol: "x86_v4_first_ge",
        target_arch: "x86_64",
        deny: &[Instruction::Call, Instruction::Div, Instruction::Idiv],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 1.8,
        }),
    },
    Gate {
        symbol: "bitpatch_apply",
        target_arch: "x86_64",
        deny: &[Instruction::Call, Instruction::Div, Instruction::Idiv],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 0.8,
        }),
    },
    Gate {
        symbol: "bitmap64_pop_lowest",
        target_arch: "x86_64",
        deny: &[Instruction::Call, Instruction::Div, Instruction::Idiv],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 1.6,
        }),
    },
    Gate {
        symbol: "fast_divisor_u32_div",
        target_arch: "x86_64",
        deny: &[Instruction::Call, Instruction::Div, Instruction::Idiv],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 3.0,
        }),
    },
    Gate {
        symbol: "power_of_two_align",
        target_arch: "x86_64",
        deny: &[Instruction::Call, Instruction::Div, Instruction::Idiv],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 1.0,
        }),
    },
    Gate {
        symbol: "pool_get_checked",
        target_arch: "x86_64",
        deny: &[Instruction::Call, Instruction::Div, Instruction::Idiv],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 1.5,
        }),
    },
    Gate {
        symbol: "pool_get_masked",
        target_arch: "x86_64",
        deny: &[Instruction::Call, Instruction::Div, Instruction::Idiv],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: Some(McaBound {
            cpu: "znver4",
            max_block_reciprocal_throughput: 1.0,
        }),
    },
];

#[unsafe(no_mangle)]
#[cold]
#[inline(never)]
pub extern "C" fn cold_sink_fn(x: u64) -> u64 {
    cold_sink_body(x)
}

// The policy macro requires Rust ABI; the exported C entry forwards to this sink.
#[uarch_macros::cold_sink]
#[unsafe(no_mangle)]
pub fn cold_sink_body(x: u64) -> u64 {
    core::hint::black_box(x)
}
