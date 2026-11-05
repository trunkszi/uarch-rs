// Covered through the public gate and workspace scenario suites.

#[test]
fn ast_grep_rejects_production_syntax() -> xtask::Result<()> {
    if std::process::Command::new("ast-grep")
        .arg("--version")
        .output()
        .is_err()
    {
        return Ok(());
    }
    let root = std::env::temp_dir().join(format!("uarch-ast-grep-{}", std::process::id()));
    let source = root.join("crates/sample/src");
    std::fs::create_dir_all(&source)?;
    let path = source.join("lib.rs");
    for text in [
        "async fn work() {}",
        "async unsafe fn work() -> u8 { 0 }",
        "fn work(v: Option<u8>) { v.unwrap(); }",
        "fn work(v: Option<u8>) { v.expect(\"value\"); }",
    ] {
        std::fs::write(&path, text)?;
        assert!(xtask::lint::ast_grep_sources(&root).is_err(), "{text}");
    }
    std::fs::write(
        &path,
        "// .unwrap() is prose.\nfn work(v: Option<u8>) -> u8 { v.unwrap_or(0) }",
    )?;
    xtask::lint::ast_grep_sources(&root)?;
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn missing_ast_grep_reports_not_run() -> xtask::Result<()> {
    if std::env::var_os("UARCH_TEST_MISSING_AST_GREP").is_some() {
        return xtask::lint::ast_grep_sources(std::path::Path::new("unused"));
    }
    let output = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "lint::missing_ast_grep_reports_not_run",
            "--nocapture",
        ])
        .env("PATH", "")
        .env("UARCH_TEST_MISSING_AST_GREP", "1")
        .output()?;
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("lint: NotRun: ast-grep unavailable"));
    Ok(())
}
