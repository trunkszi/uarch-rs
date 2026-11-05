use uarch_probes::{AbiRule, Gate, Instruction};
use xtask::audit::{audit_asm, audit_ir, mca_region, mca_throughput};

#[test]
fn malformed_artifacts_fail_closed() -> xtask::Result<()> {
    assert!(mca_region("# LLVM-MCA-BEGIN op\n addq %rax, %rbx", "op").is_err());
    let marked = "# LLVM-MCA-BEGIN op\n addq %rax, %rbx\n# LLVM-MCA-END op\n";
    assert_eq!(mca_region(marked, "op")?, " addq %rax, %rbx\n");
    assert!(mca_region(&format!("{marked}{marked}"), "op").is_err());
    assert!(mca_throughput("Block RThroughput: NaN").is_err());
    assert!(mca_throughput("Block RThroughput: -1").is_err());
    assert_eq!(mca_throughput("Block RThroughput: 1.5")?, 1.5);
    let gate = Gate {
        symbol: "f",
        target_arch: "x86_64",
        deny: &[Instruction::Jcc, Instruction::Call, Instruction::Movabs],
        require: &[],
        max_instructions: Some(4),
        abi: AbiRule::Registers,
        section: None,
        mca: None,
    };
    assert!(
        audit_ir(
            "define void @f(\n ptr sret(i64) %out\n) {\n ret void\n}",
            &[gate]
        )
        .is_err()
    );
    for mnemonic in ["jne", "callq", "movabsq"] {
        let gate = Gate {
            symbol: "f",
            target_arch: "x86_64",
            deny: &[Instruction::Jcc, Instruction::Call, Instruction::Movabs],
            require: &[],
            max_instructions: Some(4),
            abi: AbiRule::Unchecked,
            section: None,
            mca: None,
        };
        assert!(
            audit_asm(
                &format!("00000000 <f>:\n 0: {mnemonic} 0x10\n 4: retq"),
                &[gate]
            )
            .is_err()
        );
    }
    Ok(())
}

#[test]
fn indirect_values_and_tail_calls_fail() {
    let gate = Gate {
        symbol: "reg_bad",
        target_arch: "x86_64",
        deny: &[],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Registers,
        section: None,
        mca: None,
    };
    let ir = "define noundef i64 @reg_bad(ptr noalias noundef readonly align 8 captures(none) dead_on_return dereferenceable(16) %x) {\n ret i64 0\n}";
    assert!(audit_ir(ir, &[gate]).is_err());
    let gate = Gate {
        symbol: "f",
        target_arch: "x86_64",
        deny: &[Instruction::Call],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: None,
    };
    assert!(audit_asm("00000000 <f>:\n 0: jmp __pext_u64", &[gate]).is_err());
}

#[test]
fn float_divide_is_not_integer() -> xtask::Result<()> {
    let gate = Gate {
        symbol: "f",
        target_arch: "x86_64",
        deny: &[Instruction::Div, Instruction::Idiv],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: None,
    };
    audit_asm("00000000 <f>:\n 0: divsd %xmm0, %xmm1\n 4: retq", &[gate])?;
    Ok(())
}

#[test]
fn local_jumps_remain_local() -> xtask::Result<()> {
    let gate = Gate {
        symbol: "f",
        target_arch: "x86_64",
        deny: &[Instruction::Call],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: None,
    };
    audit_asm(
        "00000000 <f>:\n 0: jmp 0x4 <f+0x4>\n 4: retq",
        &[Gate { ..gate }],
    )?;
    audit_asm(
        "00000000 <f>:\n 0: jmp .LBB0_1\n 4: retq",
        &[Gate { ..gate }],
    )?;
    assert!(
        audit_asm(
            "00000000 <f>:\n 0: jmp 0x4 <f+0x4>\n 1: R_X86_64_PLT32 __pext_u64-4\n 4: retq",
            &[gate]
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn pure_rust_mca_body_is_extracted() -> xtask::Result<()> {
    let asm = ".type f,@function\nf:\n.cfi_startproc\n.LBB0_0:\n movq %rdi, %rax\n.prefalign 4, .Lfunc_end0, nop\n retq\n.Lfunc_end0:\n.size f, .Lfunc_end0-f\n";
    let body = mca_region(asm, "f")?;
    assert!(body.contains(".LBB0_0:"));
    assert!(body.contains("movq"));
    assert!(!body.contains(".prefalign"));
    assert!(!body.contains(".cfi"));
    assert!(!body.contains(".size"));
    let explicit = "# LLVM-MCA-BEGIN f\n.prefalign 4, .Lfunc_end0, nop\n retq\n# LLVM-MCA-END f";
    assert_eq!(mca_region(explicit, "f")?, " retq\n");
    assert!(mca_region("f:\n retq", "f").is_err());
    Ok(())
}

#[test]
fn arm_integer_divide_is_detected() -> xtask::Result<()> {
    for mnemonic in ["udiv", "sdiv"] {
        let gate = Gate {
            symbol: "f",
            target_arch: "aarch64",
            deny: &[Instruction::Div],
            require: &[],
            max_instructions: None,
            abi: AbiRule::Unchecked,
            section: None,
            mca: None,
        };
        assert!(
            audit_asm(
                &format!("00000000 <f>:\n 0: {mnemonic} x0, x1, x2\n 4: ret"),
                &[gate]
            )
            .is_err()
        );
    }
    let gate = Gate {
        symbol: "f",
        target_arch: "aarch64",
        deny: &[Instruction::Idiv],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: None,
    };
    assert!(
        audit_asm(
            "00000000 <f>:\n 0: sdiv x0, x1, x2\n 4: ret",
            &[Gate { ..gate }]
        )
        .is_err()
    );
    audit_asm("00000000 <f>:\n 0: fdiv d0, d1, d2\n 4: ret", &[gate])?;
    Ok(())
}

#[test]
fn pointer_boundaries_are_rejected() {
    let gate = Gate {
        symbol: "reg_bad",
        target_arch: "x86_64",
        deny: &[],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Registers,
        section: None,
        mca: None,
    };
    for signature in [
        "define i64 @reg_bad(ptr %x)",
        "define ptr @reg_bad(i64 %x)",
        "define i64 @reg_bad(ptr addrspace(1) %x)",
    ] {
        let ir = format!("{signature} {{\n unreachable\n}}");
        assert!(audit_ir(&ir, &[Gate { ..gate }]).is_err(), "{signature}");
    }
}

#[test]
fn external_tail_jumps_are_rejected() -> xtask::Result<()> {
    let gate = Gate {
        symbol: "f",
        target_arch: "x86_64",
        deny: &[Instruction::Call],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Unchecked,
        section: None,
        mca: None,
    };
    for instruction in ["jmp 4b <__pext_u64>", "b __pext_u64", "br x16"] {
        assert!(
            audit_asm(
                &format!("00000000 <f>:\n 0: {instruction}"),
                &[Gate { ..gate }]
            )
            .is_err(),
            "{instruction}"
        );
    }
    for instruction in ["jmp 4b <f+0x4b>", "b 4b <f+0x4b>", "jmp 1f"] {
        audit_asm(
            &format!("00000000 <f>:\n 0: {instruction}\n 4b: ret"),
            &[Gate { ..gate }],
        )?;
    }
    Ok(())
}

fn gem5_fixture(prefix: &str, n: f64, modern: bool) -> String {
    let insts = if modern {
        "commitStats0.numInsts"
    } else {
        "commit.instsCommitted::total"
    };
    let ops = if modern {
        "commitStats0.numOps"
    } else {
        "commit.opsCommitted::total"
    };
    format!(
        "{prefix}.numCycles {}\n{prefix}.{insts} {}\n{prefix}.{ops} {}\n{prefix}.rename.renamedInsts {}\n{prefix}.rename.idleCycles 0\n{prefix}.rename.squashCycles 0\n{prefix}.commit.branchMispredicts 0\n{prefix}.iew.memOrderViolationEvents 0\n",
        100.0 + n * 10.0,
        20.0 + n * 3.0,
        30.0 + n * 4.0,
        30.0 + n * 5.0
    )
}

#[test]
fn gem5_schemas_and_deltas() -> xtask::Result<()> {
    use xtask::audit::*;
    for (prefix, modern) in [
        ("system.cpu", false),
        ("board.processor.cores.core", true),
        ("board.processor.cores0.core", true),
    ] {
        let r0 = parse_gem5_stats(&gem5_fixture(prefix, 512.0, modern))?;
        let r1 = parse_gem5_stats(&gem5_fixture(prefix, 2560.0, modern))?;
        let r2 = parse_gem5_stats(&gem5_fixture(prefix, 4608.0, modern))?;
        let d = check_gem5_linearity(&r0, 512, &r1, 2560, &r2, 4608, 0.05)?;
        let m = compute_gem5_metrics(&d, 2048, 6)?;
        assert_eq!(m.cycles_per_op, 10.0);
        assert_eq!(m.insts_per_op, 3.0);
        assert_eq!(m.uops_per_op, 4.0);
        assert_eq!(m.attribution_mode, "o3-rename-slot");
        assert!(
            (m.retiring_pct + m.bad_speculation_pct + m.frontend_bound_pct + m.backend_bound_pct
                - 100.0)
                .abs()
                < 1e-9
        );
        assert!(diff_gem5_counters(&r0, &r1).is_err());
        let mut bad = r2;
        bad.num_cycles *= 2.0;
        assert!(check_gem5_linearity(&r0, 512, &r1, 2560, &bad, 4608, 0.05).is_err());
        let mut pressure = d;
        pressure.rob_full_events = 1e9;
        pressure.iq_full_events = 1e9;
        assert_eq!(
            compute_gem5_metrics(&pressure, 2048, 6)?.backend_bound_pct,
            m.backend_bound_pct
        );
        pressure.rename_idle_cycles = 1e9;
        assert!(compute_gem5_metrics(&pressure, 2048, 6).is_err());
    }
    Ok(())
}

#[test]
fn gem5_parser_fails_closed() {
    use xtask::audit::*;
    let good = gem5_fixture("system.cpu", 512.0, true);
    for source in [
        String::new(),
        good.replace("numCycles 5220", "numCycles NaN"),
        good.replace("numCycles 5220", "numCycles -1"),
        good.replace("numCycles 5220", "numCycles inf"),
        good.replace("commitStats0.numOps", "unknown"),
        format!("{good}{good}"),
        format!(
            "{good}{}",
            gem5_fixture("board.processor.cores.core", 512.0, true)
        ),
    ] {
        assert!(parse_gem5_stats(&source).is_err(), "{source}");
    }
    for input in [
        "",
        ",znver4",
        "znver4,",
        "znver4,,znver5",
        "unknown",
        "znver4;echo",
    ] {
        assert!(gem5_cpu_models(Some(input)).is_err(), "{input}");
    }
}

#[test]
fn gem5_aliases_and_checksum_order() -> xtask::Result<()> {
    use xtask::audit::*;
    assert_eq!(
        gem5_cpu_models(Some("zen4-o3,znver5,sapphirerapids,neoverse-v2"))?,
        vec![
            "o3-w6-rob320",
            "o3-w8-rob448",
            "o3-w6-rob512",
            "o3-a64-w6-rob320"
        ]
    );
    assert_ne!(
        combine_probe_checksum(combine_probe_checksum(0, 0, 1), 1, 2),
        combine_probe_checksum(combine_probe_checksum(0, 0, 2), 1, 1)
    );
    Ok(())
}

#[test]
fn gem5_attribution_inputs_are_explicit() -> xtask::Result<()> {
    use xtask::audit::*;
    let source = gem5_fixture("system.cpu", 1.0, true);
    let counters = parse_gem5_stats(&source)?;
    let legacy = format!("{source}system.cpu.committedInsts 1\nsystem.cpu.committedOps 1\n");
    assert_eq!(
        parse_gem5_stats(&legacy)?.committed_ops,
        counters.committed_ops
    );
    let absent = parse_gem5_stats(&source.replace("rename.idleCycles", "unused"))?;
    assert!(compute_gem5_metrics(&absent, 1, 6).is_err());
    for input in [
        format!("{source}system.cpu1.numCycles 10\n"),
        format!("---------- Begin Simulation Statistics ----------\n{source}"),
        source.replace("rename.renamedInsts 35", "rename.renamedInsts -1"),
    ] {
        assert!(parse_gem5_stats(&input).is_err());
    }
    assert!(check_gem5_linearity(&counters, 1, &counters, 1, &counters, 2, 0.05).is_err());
    assert!(check_gem5_linearity(&counters, 1, &counters, 2, &counters, 3, f64::NAN).is_err());
    assert!(compute_gem5_metrics(&counters, 0, 6).is_err());
    assert!(compute_gem5_metrics(&counters, 1, 0).is_err());
    Ok(())
}
