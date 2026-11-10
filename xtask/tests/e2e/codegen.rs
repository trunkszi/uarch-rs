use uarch_probes::{AbiRule, Gate};

#[test]
fn native_probe_artifacts_pass() -> xtask::Result<()> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let artifacts = xtask::codegen::compile(&root, "x86-64", false)?;
    assert!(std::fs::read(&artifacts.object)?.starts_with(b"\x7fELF"));
    if let Some(parent) = std::env::var_os("CARGO_TARGET_DIR") {
        let parent = std::path::PathBuf::from(parent);
        let parent = if parent.is_absolute() {
            parent
        } else {
            root.join(parent)
        };
        assert!(artifacts.object.starts_with(parent));
    }
    let gate = Gate {
        symbol: "cold_sink_fn",
        target_arch: "x86_64",
        deny: &[],
        require: &[],
        max_instructions: None,
        abi: AbiRule::Registers,
        section: Some(".text.unlikely"),
        mca: None,
    };
    assert_eq!(
        xtask::audit::audit_ir(&std::fs::read_to_string(&artifacts.ir)?, &[Gate { ..gate }])?,
        1
    );
    assert_eq!(
        xtask::audit::audit_asm(
            &xtask::codegen::dump(&artifacts.object)?,
            &[Gate {
                abi: AbiRule::Unchecked,
                ..gate
            }]
        )?,
        1
    );
    let region = xtask::audit::mca_region(
        &std::fs::read_to_string(&artifacts.assembly)?,
        "cold_sink_body",
    )?;
    assert!(!region.contains(".prefalign"));
    assert!(region.contains("ret"));
    Ok(())
}

#[path = "codegen/object.rs"]
mod object;

#[test]
fn mca_routes_complete_isa_groups() {
    for gate in uarch_probes::GATES.iter().filter(|gate| gate.mca.is_some()) {
        let target = xtask::codegen::mca_target(gate.symbol);
        if gate.symbol.starts_with("x86_v4_") {
            assert_eq!(target, "x86-64-v4");
        } else if gate.symbol.starts_with("x86_v3_")
            || ["uarch_arch_masked_sum", "uarch_arch_first_ge"].contains(&gate.symbol)
        {
            assert_eq!(target, "x86-64-v3");
        } else {
            assert_eq!(target, "x86-64");
        }
    }
    for symbol in [
        "x86_v3_scatter",
        "x86_v4_gather",
        "x86_v4_masked_sum",
        "x86_v4_first_ge",
        "bitpatch_apply",
        "bitmap64_pop_lowest",
        "fast_divisor_u32_div",
        "power_of_two_align",
        "pool_get_checked",
        "pool_get_masked",
    ] {
        assert!(
            uarch_probes::GATES
                .iter()
                .any(|gate| gate.symbol == symbol && gate.mca.is_some())
        );
    }
}

#[test]
fn optimized_kernels_emit_mask_operations() -> xtask::Result<()> {
    if !cfg!(target_arch = "x86_64") {
        return Ok(());
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let artifacts = xtask::codegen::compile(&root, "x86-64", false)?;
    let dump = xtask::codegen::dump(&artifacts.object)?;
    for symbol in ["uarch_arch_scalar_gather", "x86_v3_slow_pdep_gather"] {
        let gate = uarch_probes::GATES
            .iter()
            .find(|gate| gate.symbol == symbol)
            .ok_or_else(|| xtask::Error::Gate(format!("missing gate {symbol}")))?;
        assert_eq!(xtask::audit::audit_asm(&dump, &[Gate { ..*gate }])?, 1);
    }
    let checked = xtask::audit::instructions(&dump, "pool_get_checked")?;
    assert_eq!(
        checked
            .iter()
            .filter(|line| line.starts_with("cmp"))
            .count(),
        1,
        "{checked:?}"
    );
    let sum = xtask::audit::instructions(&dump, "x86_v4_masked_sum")?.join("\n");
    assert!(sum.contains("%k"), "{sum}");
    assert!(sum.contains("{z}"), "{sum}");
    assert!(!sum.contains("vpand"), "{sum}");
    let find = xtask::audit::instructions(&dump, "x86_v4_first_ge")?.join("\n");
    assert!(find.contains("kmov"), "{find}");
    assert!(!find.contains("movmsk"), "{find}");
    Ok(())
}
