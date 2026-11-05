use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn rejected(result: xtask::Result<()>) -> xtask::Error {
    match result {
        Err(error) => error,
        Ok(()) => panic!("invalid fixture was accepted"),
    }
}

impl Fixture {
    fn new() -> xtask::Result<Self> {
        let fixture = Self(std::env::temp_dir().join(format!(
            "uarch-lint-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )));
        std::fs::create_dir(&fixture.0)?;
        std::fs::create_dir(fixture.0.join("src"))?;
        std::fs::create_dir(fixture.0.join("tests"))?;
        std::fs::create_dir(fixture.0.join("benches"))?;
        std::fs::write(
            fixture.0.join("Cargo.toml"),
            "[workspace]\n[workspace.package]\nlicense = \"MIT\"\n[package]\nlicense.workspace = true\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )?;
        std::fs::write(
            fixture.0.join("Cargo.lock"),
            "version = 4\n[[package]]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
        )?;
        for name in ["units", "e2e", "lifecycle", "scenario"] {
            std::fs::write(fixture.0.join(format!("tests/{name}.rs")), "")?;
        }
        std::fs::write(fixture.0.join("benches/main.rs"), "fn main() {}")?;
        std::fs::write(fixture.0.join("src/lib.rs"), "pub fn stable() {}")?;
        Ok(fixture)
    }
    fn source(&self, text: &str) -> xtask::Result<()> {
        std::fs::write(self.0.join("src/lib.rs"), text)?;
        xtask::lint::check_workspace(&self.0)
    }
}

#[test]
fn lint_rejects_structural_violations() -> xtask::Result<()> {
    let fixture = Fixture::new()?;
    let source = fixture.0.join("src/lib.rs");
    std::fs::write(
        &source,
        "pub fn stable() { let _ = \"async fn bad() {}\"; }\n",
    )?;
    xtask::lint::check_workspace(&fixture.0)?;
    let manifest = fixture.0.join("Cargo.toml");
    let valid_manifest = std::fs::read_to_string(&manifest)?;
    for invalid_manifest in [
        valid_manifest.replace("MIT", "Apache-2.0"),
        valid_manifest.replace("license.workspace = true", "license = \"MIT\""),
        format!(
            "{}\n[package.metadata]\nlicense.workspace = true\n",
            valid_manifest.replace("license.workspace = true", "license = \"MIT\"")
        ),
    ] {
        std::fs::write(&manifest, invalid_manifest)?;
        assert!(xtask::lint::check_workspace(&fixture.0).is_err());
    }
    std::fs::write(&manifest, valid_manifest)?;

    for invalid in [
        "pub fn bad() { BitFlag::new(1); }".to_owned(),
        "pub fn bad() { BitFlag::<u64>::new(1); }".to_owned(),
        "pub fn bad() { uarch_core::bits::BitField::<W,u8,0,1>::new(); }".to_owned(),
        "pub async fn bad() {}".to_owned(),
        "pub fn bad() { let _ = Some(1).unwrap(); }".to_owned(),
        "pub fn bad() { let _ = P.and_flag(FLAG); }".to_owned(),
        format!(
            "pub const TEXT: &str = \"{}\";",
            char::from_u32(0x4e2d).unwrap_or('x')
        ),
    ] {
        std::fs::write(&source, invalid)?;
        assert!(xtask::lint::check_workspace(&fixture.0).is_err());
    }
    std::fs::write(&source, "pub fn stable() {}")?;
    for file in ["tests/descriptor.rs", "benches/descriptor.rs"] {
        std::fs::write(
            fixture.0.join(file),
            "fn descriptor() { BitFlag::<u64>::new(1); BitField::<W,u8,0,1>::new(); }",
        )?;
    }
    xtask::lint::check_workspace(&fixture.0)?;
    let ui = fixture.0.join("tests/ui/fail/input.rs");
    std::fs::create_dir_all(
        ui.parent()
            .ok_or_else(|| xtask::Error::Gate("missing UI parent".into()))?,
    )?;
    std::fs::write(
        &ui,
        "async fn an_intentionally_long_fixture_name_with_more_than_six_segments() { BitFlag::new(1); }",
    )?;
    xtask::lint::check_workspace(&fixture.0)?;
    std::fs::write(&ui, "\n".repeat(801))?;
    assert!(xtask::lint::check_workspace(&fixture.0).is_err());
    std::fs::remove_file(&ui)?;
    let macro_source = fixture.0.join("crates/uarch-core/src/bits/word_macro.rs");
    std::fs::create_dir_all(
        macro_source
            .parent()
            .ok_or_else(|| xtask::Error::Gate("missing macro parent".into()))?,
    )?;
    std::fs::write(&macro_source, "fn descriptor() { BitFlag::<u64>::new(1); }")?;
    xtask::lint::check_workspace(&fixture.0)?;
    std::fs::remove_file(fixture.0.join("tests/e2e.rs"))?;
    assert!(xtask::lint::check_workspace(&fixture.0).is_err());
    Ok(())
}

#[test]
fn lint_checks_deferred_and_macro_code() -> xtask::Result<()> {
    let fixture = Fixture::new()?;
    for (source, reason) in [
        ("const F: fn(P)->P = |p| p.and_flag(FLAG);", "const context"),
        (
            "const F: fn(P)->P = { fn f(p:P)->P { p.and_flag(FLAG) } f };",
            "const context",
        ),
        (
            "const X: () = { struct S; impl S { fn f(p:P)->P { p.and_flag(FLAG) } } };",
            "const context",
        ),
        (
            "const X: () = { trait T { fn f(p:P)->P { p.and_flag(FLAG) } } };",
            "const context",
        ),
        (
            "pub fn bad() { let _ = async || 1; }",
            "asynchronous Rust syntax",
        ),
        (
            "pub fn bad() { let _ = Option::unwrap(Some(1)); }",
            "unchecked production extraction",
        ),
        (
            "pub fn bad() { let _ = Result::expect(Ok(1), \"missing\"); }",
            "unchecked production extraction",
        ),
        (
            "pub fn bad() { println!(\"{}\", Some(1).unwrap()); }",
            "unchecked production extraction",
        ),
        (
            "macro_rules! m { () => { pub async fn bad() {} } }",
            "asynchronous Rust syntax",
        ),
        (
            "macro_rules! m { () => { value.await } }",
            "asynchronous Rust syntax",
        ),
        (
            "macro_rules! m { () => { BitFlag::new(1) } }",
            "bit descriptors",
        ),
        (
            "macro_rules! m { () => { BitField::<W,u8,0,1>::new() } }",
            "bit descriptors",
        ),
        ("opaque!(model_zoo);", "forbidden domain identifier"),
        (
            "#[cfg_attr(all(), target_feature(enable = \"avx2\"))] pub unsafe fn bad() {}",
            "target_feature",
        ),
    ] {
        let error = rejected(fixture.source(source));
        assert!(error.to_string().contains(reason), "{source}: {error}");
    }
    for source in [
        "const X: P = P.and_flag(FLAG);",
        "static X: P = P.and_flag(FLAG);",
        "fn f() { let x = const { P.and_flag(FLAG) }; }",
        "impl S { const X: P = P.and_flag(FLAG); }",
        "trait T { const X: P = P.and_flag(FLAG); }",
        "opaque!(\"async fn venue() { None.unwrap() }\");",
        "opaque!(BitFlag, BitField, unwrap(value), expect(value));",
    ] {
        fixture.source(source)?;
    }
    let han = char::from_u32(0x4e2d).unwrap_or('x');
    for source in [
        format!("const S: &str = \"{han}\";"),
        format!("opaque!(\"{han}\");"),
    ] {
        let error = rejected(fixture.source(&source));
        assert!(
            error
                .to_string()
                .contains("Chinese text belongs in Markdown documentation, not Rust source")
        );
    }
    Ok(())
}

#[test]
fn lint_rejects_renamed_async_packages() -> xtask::Result<()> {
    let fixture = Fixture::new()?;
    let manifest = fixture.0.join("Cargo.toml");
    let valid = std::fs::read_to_string(&manifest)?;
    for package in [
        "tokio",
        "futures-core",
        "futures-util",
        "futures-lite",
        "async-io",
        "async-trait",
        "glommio",
        "monoio",
        "embassy",
    ] {
        std::fs::write(
            &manifest,
            format!(
                "{valid}\n[dependencies]\nrt = {{ version = \"1\", package = \"{package}\" }}\n"
            ),
        )?;
        let error = rejected(xtask::lint::check_workspace(&fixture.0));
        assert!(
            error
                .to_string()
                .contains("asynchronous runtime dependency"),
            "{package}: {error}"
        );
    }
    Ok(())
}

#[test]
fn lint_checks_attribute_literals() -> xtask::Result<()> {
    let fixture = Fixture::new()?;
    let han = char::from_u32(0x4e2d).unwrap_or('x');
    for source in [
        format!("#[deprecated(note = \"{han}\")] pub fn bad() {{}}"),
        format!("#[error(\"{han}\")] struct Bad;"),
        format!("#[cfg_attr(any(), deprecated(note = \"{han}\"))] fn bad() {{}}"),
        format!("#[custom('{han}')] struct Bad;"),
    ] {
        let error = rejected(fixture.source(&source));
        assert!(
            error
                .to_string()
                .contains("Chinese text belongs in Markdown documentation, not Rust source"),
            "{source}: {error}"
        );
    }
    fixture.source("#[deprecated(note = \"English text\")] fn stable() {}")?;
    Ok(())
}

#[test]
fn lint_rejects_chinese_source_text() -> xtask::Result<()> {
    let fixture = Fixture::new()?;
    let han = char::from_u32(0x4e2d).unwrap_or('x');
    for source in [
        format!("/// {han}\nfn bad() {{}}"),
        format!("//! {han}\nfn bad() {{}}"),
        format!("#[doc = \"{han}\"] fn bad() {{}}"),
        format!("#[cfg_attr(all(), doc = \"{han}\")] fn bad() {{}}"),
        format!("// {han}\nfn bad() {{}}"),
        format!("/* {han} */ fn bad() {{}}"),
        format!("fn {han}() {{}}"),
        format!("const C: char = '{han}';"),
        r"const C: char = '\u{4e2d}';".to_owned(),
        r"opaque!('\u{4e2d}');".to_owned(),
        format!("macro_rules! m {{ () => {{ /// {han}\nfn bad() {{}} }} }}"),
    ] {
        let error = rejected(fixture.source(&source));
        assert!(
            error
                .to_string()
                .contains("Chinese text belongs in Markdown documentation, not Rust source"),
            "{source}: {error}"
        );
    }
    fixture.source("//! English documentation\n/// English description\nfn stable() {}")?;
    fixture.source("// English comment\nconst C: char = 'x';")?;
    Ok(())
}

#[test]
fn lint_rejects_cjk_punctuation() -> xtask::Result<()> {
    let fixture = Fixture::new()?;
    for codepoint in [
        0x3000, 0x303f, 0x3400, 0x4dbf, 0x4e00, 0x9fff, 0xf900, 0xfaff, 0xff00, 0xff0c, 0xffef,
        0x20000, 0x323af,
    ] {
        let punctuation = char::from_u32(codepoint).unwrap_or('x');
        let error = rejected(fixture.source(&format!("/// {punctuation}\nfn bad() {{}}")));
        assert!(
            error
                .to_string()
                .contains("Chinese text belongs in Markdown documentation, not Rust source")
        );
    }
    let error = rejected(fixture.source(r"const C: char = '\u{ff0c}';"));
    assert!(
        error
            .to_string()
            .contains("Chinese text belongs in Markdown documentation, not Rust source")
    );
    Ok(())
}
