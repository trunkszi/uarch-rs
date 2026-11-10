//! Synchronous subprocess execution with checked exit status.
use crate::{Error, Result};
use std::process::{Command, Output};
/// Captures a successful command.
/// # Errors
/// Rejects spawn or nonzero exit status.
pub fn output(command: &mut Command) -> Result<Output> {
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

/// Executes a command inheriting its configured streams.
/// # Errors
/// Rejects spawn or nonzero exit status.
pub fn run(command: &mut Command) -> Result<()> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Gate(format!(
            "command {command:?} failed ({status})"
        )))
    }
}

/// Captures UTF-8 standard output.
/// # Errors
/// Rejects a failed command or invalid UTF-8.
pub fn text(command: &mut Command) -> Result<String> {
    String::from_utf8(output(command)?.stdout)
        .map_err(|error| Error::Gate(format!("tool output is not UTF-8: {error}")))
}
