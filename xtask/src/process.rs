use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) const PINNED: &str = "";
pub(crate) const MSRV: &str = "+1.95.0";

pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

pub(crate) use uarch_audit::process::{output, run, text};

pub(crate) fn cargo(toolchain: &str, root: &Path) -> Command {
    let mut command = Command::new("cargo");
    if !toolchain.is_empty() {
        command.arg(toolchain);
    }
    command.current_dir(root);
    command
}
