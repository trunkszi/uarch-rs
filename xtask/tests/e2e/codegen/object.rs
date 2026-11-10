// Covered by the public codegen artifact and parser regression suites.

use uarch_probes::{AbiRule, Gate, Instruction};

fn leaf() -> Gate {
    Gate {
        symbol: "leaf",
        target_arch: "x86_64",
        deny: &[Instruction::Call],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Registers,
        section: None,
        mca: None,
    }
}

#[test]
fn nm_rejects_missing_zero_and_duplicates() -> xtask::Result<()> {
    assert_eq!(
        xtask::codegen::symbol_sizes("leaf T 0 10\n", &[leaf()])?["leaf"],
        16
    );
    for text in [
        "",
        "leaf T 0 0",
        "leaf U 0 10",
        "leaf T 0 10\nleaf T 0 10",
        "leaf T nope 10",
    ] {
        assert!(
            xtask::codegen::symbol_sizes(text, &[leaf()]).is_err(),
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn elf_rejects_wrong_sections_and_relocations() -> xtask::Result<()> {
    let report = "Sections [\n]\nSymbols [\n  Symbol {\n    Name: leaf (1)\n    Value: 0x0\n    Size: 16\n    Section: .text.leaf (1)\n  }\n]\nRelocations [\n]\n";
    xtask::codegen::readobj_report(report, &[leaf()])?;
    let relocated = report.replace(
        "Relocations [\n]",
        "Relocations [\n Section (2) .rela.text.leaf {\n  0x8 R_X86_64_GOTPCREL helper 0x0\n }\n]",
    );
    assert!(xtask::codegen::readobj_report(&relocated, &[leaf()]).is_err());
    let outside = relocated.replace("0x8 R_X86", "0x20 R_X86");
    xtask::codegen::readobj_report(&outside, &[leaf()])?;
    let cold = Gate {
        section: Some(".text.unlikely"),
        ..leaf()
    };
    assert!(xtask::codegen::readobj_report(report, &[Gate { ..cold }]).is_err());
    xtask::codegen::readobj_report(
        &report.replace(".text.leaf", ".text.unlikely.leaf"),
        &[cold],
    )?;
    assert!(xtask::codegen::readobj_report("", &[leaf()]).is_err());
    Ok(())
}

#[test]
fn elf_accepts_constant_pool_relocations() -> xtask::Result<()> {
    let report = "Sections [\n]\nSymbols [\n  Symbol {\n    Name: leaf (1)\n    Value: 0x0\n    Size: 16\n    Section: .text.leaf (1)\n  }\n]\nRelocations [\n Section (2) .rela.text.leaf {\n  0x8 R_X86_64_PC32 .rodata.cst16 0x0\n }\n]\n";
    xtask::codegen::readobj_report(report, &[leaf()])?;
    xtask::codegen::readobj_report(&report.replace(".rodata.cst16", ".rodata-0x4"), &[leaf()])?;
    assert!(
        xtask::codegen::readobj_report(&report.replace(".rodata.cst16", "helper"), &[leaf()])
            .is_err()
    );
    assert!(
        xtask::codegen::readobj_report(
            &report.replace("R_X86_64_PC32", "R_X86_64_GOTPCREL"),
            &[leaf()]
        )
        .is_err()
    );
    Ok(())
}
