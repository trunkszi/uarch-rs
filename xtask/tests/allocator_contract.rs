#[test]
fn xtask_binaries_link_mimalloc() -> xtask::Result<()> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for file in ["src/main.rs", "benches/main.rs"] {
        uarch_audit::allocator::verify_source(&std::fs::read_to_string(root.join(file))?)?;
    }
    let nm = xtask::codegen::find_llvm_tool("llvm-nm")?;
    uarch_audit::allocator::verify_binary(&nm, std::path::Path::new(env!("CARGO_BIN_EXE_xtask")))
}
