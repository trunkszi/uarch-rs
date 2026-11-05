use crate::{Error, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub(crate) const PINNED: &str = "";
pub(crate) const MSRV: &str = "+1.95.0";

pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

pub(crate) fn output(command: &mut Command) -> Result<Output> {
    let result = command.output()?;
    if !result.status.success() {
        return Err(Error::Gate(format!(
            "command {command:?} failed ({}):\n{}\n{}",
            result.status,
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        )));
    }
    Ok(result)
}

pub(crate) fn run(command: &mut Command) -> Result<()> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Gate(format!(
            "command {command:?} failed ({status})"
        )))
    }
}

pub(crate) fn cargo(toolchain: &str, root: &Path) -> Command {
    let mut command = Command::new("cargo");
    if !toolchain.is_empty() {
        command.arg(toolchain);
    }
    command.current_dir(root);
    command
}

pub(crate) fn text(command: &mut Command) -> Result<String> {
    String::from_utf8(output(command)?.stdout)
        .map_err(|error| Error::Gate(format!("tool output is not UTF-8: {error}")))
}
