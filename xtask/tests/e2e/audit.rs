use uarch_probes::{AbiRule, Gate, Instruction};
use xtask::audit::{audit_asm, audit_ir, audit_sections};

fn rule(symbol: &'static str, abi: AbiRule) -> Gate {
    Gate {
        symbol,
        target_arch: "x86_64",
        deny: &[],
        require: &[],
        max_instructions: None,
        abi,
        section: None,
        mca: None,
    }
}

#[test]
fn ir_checks_registers_and_control() -> xtask::Result<()> {
    let gates = [
        rule("pair", AbiRule::Registers),
        rule("pressure", AbiRule::MemoryControl),
    ];
    let ir = "define { i64, i64 } @pair(i64 %a, i64 %b) {\n ret { i64, i64 } zeroinitializer\n}\n\
              define void @pressure(ptr byval([2 x i64]) %v) {\n ret void\n}\n";
    assert_eq!(audit_ir(ir, &gates)?, 2);
    assert!(audit_ir(&ir.replace("byval([2 x i64])", "nonnull"), &gates).is_err());
    assert!(audit_ir(&ir.replace("i64 %a", "ptr sret(i64) %a"), &gates).is_err());
    assert!(audit_ir("declare i64 @pair(i64)", &gates).is_err());
    let split =
        "define { i64, i64 } @pair(\n ptr byval(i64) %a) {\n ret { i64, i64 } zeroinitializer\n}\n";
    let gate = rule("pair", AbiRule::Registers);
    assert!(audit_ir(split, &[gate]).is_err());
    Ok(())
}

#[test]
fn asm_checks_full_symbol() -> xtask::Result<()> {
    let gate = Gate {
        deny: &[Instruction::Pdep],
        require: &[Instruction::Pext],
        max_instructions: Some(2),
        ..rule("gather", AbiRule::Unchecked)
    };
    let dump = "00000000 <gather>:\n 0: pextq %rdi, %rsi, %rax\n 5: retq\n\
                00000010 <other>:\n 10: pdepq %rdi, %rsi, %rax\n 15: retq\n";
    assert_eq!(audit_asm(dump, &[gate])?, 1);
    let gate = Gate {
        deny: &[Instruction::Pdep],
        ..rule("other", AbiRule::Unchecked)
    };
    assert!(audit_asm(dump, &[gate]).is_err());
    let gate = Gate {
        section: Some(".text.unlikely"),
        ..rule("sink", AbiRule::Unchecked)
    };
    audit_sections("00000000 g F .text.unlikely.sink 0008 sink", &[gate])?;
    let gate = Gate {
        section: Some(".text.unlikely"),
        ..rule("sink", AbiRule::Unchecked)
    };
    assert!(audit_sections("00000000 g F .text 0008 sink", &[gate]).is_err());
    Ok(())
}

#[test]
fn register_asm_rejects_frames() -> xtask::Result<()> {
    for instruction in [
        "pushq %rax",
        "popq %rax",
        "movq 8(%rsp), %rax",
        "movq -8(%rbp), %rax",
        "mov rax, [rbp + 8]",
        "ldr x0, [sp, #16]",
        "push {r4}",
    ] {
        let gate = rule("leaf", AbiRule::Registers);
        let dump = format!("00000000 <leaf>:\n 0: {instruction}\n 4: ret\n");
        assert!(audit_asm(&dump, &[gate]).is_err(), "{instruction}");
    }
    assert_eq!(
        audit_asm(
            "00000000 <leaf>:\n 0: movq %rdi, %rax\n 4: retq\n",
            &[rule("leaf", AbiRule::Registers)]
        )?,
        1
    );
    Ok(())
}

#[test]
fn register_asm_accepts_population_count() -> xtask::Result<()> {
    for instruction in [
        "popcnt rax, rdi",
        "popcntq %rdi, %rax",
        "rep popcntq %rdi, %rax",
    ] {
        let dump = format!("00000000 <leaf>:\n 0: {instruction}\n 4: ret\n");
        assert_eq!(audit_asm(&dump, &[rule("leaf", AbiRule::Registers)])?, 1);
    }
    Ok(())
}

#[test]
fn multiply_rule_rejects_integer_products() -> xtask::Result<()> {
    for instruction in [
        "imulq %rdi, %rax",
        "imul rax, rdi",
        "mul %rdi",
        "mulxq %rsi, %rax, %rdx",
        "mul x0, x1, x2",
        "umulh x0, x1, x2",
        "madd x0, x1, x2, x3",
        "msub x0, x1, x2, x3",
        "rep imulq %rdi, %rax",
    ] {
        let gate = Gate {
            deny: &[Instruction::Imul],
            ..rule("leaf", AbiRule::Unchecked)
        };
        let dump = format!("00000000 <leaf>:\n 0: {instruction}\n 4: ret\n");
        assert!(audit_asm(&dump, &[gate]).is_err(), "{instruction}");
    }
    let gate = Gate {
        deny: &[Instruction::Imul],
        ..rule("leaf", AbiRule::Unchecked)
    };
    assert_eq!(
        audit_asm(
            "00000000 <leaf>:\n 0: mulss %xmm0, %xmm1\n 4: retq\n",
            &[gate]
        )?,
        1
    );
    Ok(())
}

#[test]
fn gather_contract_rejects_multiplier() -> xtask::Result<()> {
    for symbol in ["uarch_arch_scalar_gather", "x86_v3_slow_pdep_gather"] {
        let gate = uarch_probes::GATES
            .iter()
            .find(|gate| gate.symbol == symbol)
            .ok_or_else(|| xtask::Error::Gate(format!("missing gate {symbol}")))?;
        let dump = format!("00000000 <{symbol}>:\n 0: imulq %rdi, %rax\n 4: retq\n");
        assert!(audit_asm(&dump, &[Gate { ..*gate }]).is_err());
    }
    Ok(())
}
