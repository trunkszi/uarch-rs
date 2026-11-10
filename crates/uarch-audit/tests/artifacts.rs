#![allow(clippy::unwrap_used)]
use std::path::Path;
use uarch_audit::artifacts::select_artifact;
#[test]
fn manifest_identity_selects_artifact() {
    let stream = r#"{"reason":"compiler-artifact","manifest_path":"/work/other/Cargo.toml","target":{"name":"probe"},"filenames":["/out/other.o"]}
{"reason":"compiler-artifact","manifest_path":"/work/probe/Cargo.toml","target":{"name":"probe"},"filenames":["/out/probe.o","/out/probe.ll","/out/probe.s"],"executable":"/out/probe"}
{"reason":"build-finished","success":true}"#;
    let selected = select_artifact(stream, Path::new("/work/probe/Cargo.toml"), "probe").unwrap();
    assert_eq!(
        selected.executable.as_deref(),
        Some(Path::new("/out/probe"))
    );
    assert_eq!(selected.filenames[0], Path::new("/out/probe.o"));
    assert!(
        select_artifact(
            &format!("{stream}\n{stream}"),
            Path::new("/work/probe/Cargo.toml"),
            "probe"
        )
        .is_err()
    );
    assert!(select_artifact(stream, Path::new("/missing/Cargo.toml"), "probe").is_err());
}

mod support;
#[test]
fn exact_sidecars_do_not_scan_siblings() {
    let dir = support::TestDir::new();
    let root = dir.path();
    for name in [
        "libprobe-aaa.rlib",
        "probe-aaa.o",
        "probe-aaa.ll",
        "probe-aaa.s",
        "probe-stale.o",
    ] {
        std::fs::write(root.join(name), "").unwrap();
    }
    let artifact = uarch_audit::artifacts::CargoArtifact {
        manifest_path: "/p/Cargo.toml".into(),
        filenames: vec![root.join("libprobe-aaa.rlib")],
        executable: None,
    };
    assert_eq!(artifact.emitted().unwrap().object, root.join("probe-aaa.o"));
    std::fs::remove_file(root.join("probe-aaa.o")).unwrap();
    assert!(artifact.emitted().is_err());
}
#[test]
fn cold_and_hot_contracts_are_falsifiable() {
    use uarch_audit::artifacts::{verify_cold_sections, verify_hot_symbol};
    let cold = vec!["cold".to_string()];
    assert!(verify_cold_sections("0 g F .text.unlikely.cold 10 cold", &cold).is_ok());
    assert!(verify_cold_sections("0 g F .text 10 cold", &cold).is_err());
    assert!(verify_cold_sections("0 g F .text.unlikelyX 10 cold", &cold).is_err());
    let dump = "0000 <hot>:\n 0: addq %rax, %rbx\n 3: retq\n";
    let ir = "define void @hot() {\n ret void\n}\n";
    assert!(verify_hot_symbol(dump, ir, "hot", false).is_ok());
    for op in ["ud2", "idivq", "divq", "int3", "sdiv"] {
        assert!(verify_hot_symbol(&dump.replace("addq", op), ir, "hot", false).is_err());
    }
    assert!(
        verify_hot_symbol(
            dump,
            &ir.replace("ret void", "%v = udiv i64 8, 2\n ret void"),
            "hot",
            false
        )
        .is_err()
    );
    assert!(
        verify_hot_symbol(
            dump,
            &ir.replace("ret void", "invoke void @f() to label %a unwind label %b"),
            "hot",
            false
        )
        .is_err()
    );
    let attr = "define void @hot() #0 {\nret void\n}\nattributes #0 = { nounwind uwtable }\n";
    assert!(verify_hot_symbol(dump, attr, "hot", false).is_err());
    assert!(verify_hot_symbol(dump, ir, "missing", false).is_err());
}

#[test]
fn prefixed_traps_are_not_hidden() {
    let ir = "define void @hot() {\nret void\n}\n";
    for prefix in ["repz", "repnz", "rex.W", "data16", "ss"] {
        let dump = format!("0000 <hot>:\n 0: {prefix} ud2\n");
        assert!(uarch_audit::artifacts::verify_hot_symbol(&dump, ir, "hot", false).is_err());
    }
}

#[test]
fn nounwind_personality_without_eh_passes() {
    use uarch_audit::artifacts::verify_hot_symbol;
    let dump = "0000 <hot>:\n 0: retq\n";
    for header in [
        "define void @hot() nounwind personality ptr @rust_eh_personality {",
        "define void @hot() #0 personality ptr @rust_eh_personality {",
    ] {
        let ir = format!("{header}\n ret void\n}}\nattributes #0 = {{ nounwind }}\n");
        assert!(verify_hot_symbol(dump, &ir, "hot", false).is_ok());
    }
}

#[test]
fn personality_still_requires_no_unwind() {
    use uarch_audit::artifacts::verify_hot_symbol;
    let dump = "0000 <hot>:\n 0: retq\n";
    let ir = "define void @hot() #0 personality ptr @rust_eh_personality {\n ret void\n}\nattributes #0 = { nounwind }\n";
    for body in [
        "invoke void @f() to label %a unwind label %b",
        "%pad = landingpad { ptr, i32 } cleanup",
        "resume { ptr, i32 } zeroinitializer",
        "call void @_Unwind_Resume(ptr null)",
    ] {
        assert!(
            verify_hot_symbol(dump, &ir.replace("ret void", body), "hot", false).is_err(),
            "{body}"
        );
    }
    for attrs in ["", "noinline", "\"nounwind\"", "nounwind uwtable"] {
        assert!(
            verify_hot_symbol(
                dump,
                &ir.replace("{ nounwind }", &format!("{{ {attrs} }}")),
                "hot",
                false
            )
            .is_err(),
            "{attrs}"
        );
    }
    assert!(verify_hot_symbol(dump, &ir.replace(" #0", ""), "hot", false).is_err());
}

#[test]
fn quoted_section_cannot_supply_nounwind() {
    let dump = "0000 <hot>:\n 0: retq\n";
    let ir = "define void @hot() section \"x nounwind y\" personality ptr @rust_eh_personality {\nret void\n}\n";
    assert!(uarch_audit::artifacts::verify_hot_symbol(dump, ir, "hot", false).is_err());
}

#[test]
fn header_comment_cannot_supply_nounwind() {
    let dump = "0000 <hot>:\n 0: retq\n";
    let ir = "define void @hot() personality ptr @p { ; nounwind\nret void\n}\n";
    assert!(uarch_audit::artifacts::verify_hot_symbol(dump, ir, "hot", false).is_err());
}

#[test]
fn quoted_attribute_cannot_supply_nounwind() {
    let dump = "0000 <hot>:\n 0: retq\n";
    let ir = "define void @hot() #0 personality ptr @p {\nret void\n}\nattributes #0 = { \"k\"=\"a nounwind b\" }\n";
    assert!(uarch_audit::artifacts::verify_hot_symbol(dump, ir, "hot", false).is_err());
}

#[test]
fn attribute_comment_cannot_supply_nounwind() {
    let dump = "0000 <hot>:\n 0: retq\n";
    let ir = "define void @hot() #0 personality ptr @p {\nret void\n}\nattributes #0 = { noinline } ; nounwind\n";
    assert!(uarch_audit::artifacts::verify_hot_symbol(dump, ir, "hot", false).is_err());
}

#[test]
fn missing_attribute_group_is_rejected() {
    let dump = "0000 <hot>:\n 0: retq\n";
    let ir =
        "define void @hot() #99 personality ptr @p {\nret void\n}\nattributes #0 = { nounwind }\n";
    assert!(uarch_audit::artifacts::verify_hot_symbol(dump, ir, "hot", false).is_err());
}

#[test]
fn adjacent_personality_requires_nounwind() {
    let dump = "0000 <hot>:\n 0: retq\n";
    let ir = "define void @hot()personality ptr @p {\nret void\n}\n";
    assert!(uarch_audit::artifacts::verify_hot_symbol(dump, ir, "hot", false).is_err());
}

#[test]
fn suffix_cannot_supply_group_nounwind() {
    let dump = "0000 <hot>:\n 0: retq\n";
    let ir = "define void @hot() #0 personality ptr @p {\nret void\n}\nattributes #0 = { noinline } nounwind\n";
    assert!(uarch_audit::artifacts::verify_hot_symbol(dump, ir, "hot", false).is_err());
}

#[test]
fn literal_delimiters_preserve_real_keywords() {
    let dump = "0000 <hot>:\n 0: retq\n";
    let ir = r#"define void @hot()#0 section "x #99 \22 ; } y" personality ptr @p {
ret void
}
attributes #0 = { "k"="a ; } \22 b" nounwind }
"#;
    assert!(uarch_audit::artifacts::verify_hot_symbol(dump, ir, "hot", false).is_ok());
}
