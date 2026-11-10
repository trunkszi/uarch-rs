#![allow(clippy::unwrap_used)]
use uarch_audit::allocator::verify_source;
const BINDING: &str = "#[global_allocator] static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;";
#[test]
fn only_unconditional_root_is_admitted() {
    assert!(verify_source(BINDING).is_ok());
    for source in [
        format!("#[cfg(any())] {BINDING}"),
        format!("#[cfg(test)] {BINDING}"),
        "#[cfg_attr(all(), global_allocator)] static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;".into(),
        format!("mod inner {{ {BINDING} }}"),
        format!("fn f() {{ {BINDING} }}"),
        format!("const _: () = {{ {BINDING} }};"),
        format!("#![cfg(test)] {BINDING}"),
        format!("{BINDING} {BINDING}"),
        format!("{BINDING} #[cfg_attr(any(), global_allocator)] static OTHER: mimalloc::MiMalloc = mimalloc::MiMalloc;"),
        BINDING.replace("mimalloc::MiMalloc =", "std::alloc::System ="),
    ] { assert!(verify_source(&source).is_err(), "accepted: {source}"); }
}

mod support;
#[test]
fn linked_symbols_must_be_defined() {
    use uarch_audit::allocator::verify_symbols;
    let valid = "__rust_alloc T 10 2\nmi_malloc_aligned T 20 3\nmi_free T 30 4\n";
    assert!(verify_symbols(valid).is_ok());
    for bad in [
        valid.replace("__rust_alloc T", "__rust_alloc U"),
        valid.replace("mi_free", "other_free"),
        valid.replace("mi_malloc_aligned", "other_malloc"),
        valid.replace("__rust_alloc", "not__rust_alloc"),
    ] {
        assert!(verify_symbols(&bad).is_err());
    }
}
#[test]
fn metadata_covers_standalone_targets() {
    let dir = support::TestDir::new();
    let root = dir.path();
    for path in [
        "src/main.rs",
        "src/bin/extra.rs",
        "examples/probe.rs",
        "benches/plain.rs",
        "benches/harness.rs",
    ] {
        let p = root.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, BINDING).unwrap();
    }
    let manifest = root.join("Cargo.toml");
    std::fs::write(&manifest,"[package]\nname='fixture'\nversion='0.1.0'\n[[bench]]\nname='plain'\nharness=false\n[[bench]]\nname='harness'\nharness=true\n").unwrap();
    let json = format!(
        r#"{{"packages":[{{"manifest_path":{:?},"targets":[{{"kind":["bin"],"src_path":{:?}}},{{"kind":["bin"],"src_path":{:?}}},{{"kind":["example"],"src_path":{:?}}},{{"kind":["bench"],"src_path":{:?}}}]}}]}}"#,
        manifest,
        root.join("src/main.rs"),
        root.join("src/bin/extra.rs"),
        root.join("examples/probe.rs"),
        root.join("benches/harness.rs")
    );
    let sources = uarch_audit::allocator::sources_from_metadata(&json).unwrap();
    assert_eq!(sources.len(), 4);
    assert!(sources.contains(&root.join("benches/plain.rs")));
    assert!(!sources.contains(&root.join("benches/harness.rs")));
}

#[test]
fn unrelated_cfg_attr_is_not_an_allocator() {
    let source =
        format!("{BINDING}\n#[cfg_attr(any(), doc = \"global_allocator\")] fn unrelated() {{}}");
    assert!(verify_source(&source).is_ok());
}
