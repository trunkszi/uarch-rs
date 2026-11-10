use crate::{Error, Result, codegen, hardware, lint, process};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Reproducible uarch validation gates")]
struct Cli {
    #[command(subcommand)]
    command: GateCommand,
}

#[derive(Subcommand)]
enum GateCommand {
    /// Check workspace formatting.
    Fmt,
    /// Check all workspace targets with warnings denied.
    Clippy,
    /// Run all workspace targets and doctests.
    Test,
    /// Check formatting, Clippy, AST rules, and repository structure.
    Lint,
    /// Check and test portable semantics on Rust 1.95.0.
    Msrv,
    /// Build all targets and execute proc-macro UI tests.
    Ui,
    /// Verify concrete LLVM signatures and a memory negative control.
    AbiCheck,
    /// Check concrete IR symbol bodies and arithmetic lowering.
    IrCheck,
    /// Audit baseline and native CPU disassembly matrices.
    AsmCheck,
    /// Report native symbol sizes in bytes.
    SizeReport,
    /// Run reproducible semantic, feature, ABI and codegen checks.
    Verify,
    /// Check LLVM versions, marker parity, and MCA throughput budgets.
    McaGate {
        /// Comma-separated LLVM CPU models; defaults to UARCH_MCA_CPU or the host matrix.
        #[arg(long)]
        cpu: Option<String>,
    },
    /// Verify parametric O3 profiles using fixed-width three-run guest measurements.
    Gem5Gate {
        /// Comma-separated canonical O3 profiles or inspired-by aliases.
        #[arg(long)]
        cpu: Option<String>,
    },
    #[command(name = "__gem5-probe", hide = true)]
    Gem5Probe {
        #[arg(long, value_enum)]
        probe: hardware::Probe,
        #[arg(long)]
        iterations: std::num::NonZeroU64,
    },
    /// Measure a named probe or a supplied benchmark executable.
    #[command(group(clap::ArgGroup::new("source").args(["executable", "probe"])))]
    PmuGate {
        #[arg(long, requires = "operations")]
        executable: Option<PathBuf>,
        #[arg(long, requires = "executable")]
        operations: Option<std::num::NonZeroU64>,
        #[arg(long, value_enum, requires = "iterations")]
        probe: Option<hardware::Probe>,
        #[arg(long, requires = "probe")]
        iterations: Option<std::num::NonZeroU64>,
        #[arg(last = true, requires = "executable")]
        args: Vec<String>,
    },
    #[command(name = "__pmu-probe", hide = true)]
    ExecuteProbe {
        #[arg(long, value_enum)]
        probe: hardware::Probe,
        #[arg(long)]
        iterations: std::num::NonZeroU64,
    },
    /// Run portable Miri tests under Stacked and Tree Borrows.
    Miri,
    /// Run the workspace benchmark executables.
    Bench,
}

/// Executes the requested gate from the workspace root.
///
/// # Errors
/// Returns an error if a tool is unavailable or any validation fails.
pub fn run() -> Result<()> {
    let root = process::root();
    match Cli::parse().command {
        GateCommand::Fmt => process::run(
            process::cargo(process::PINNED, &root).args(["fmt", "--all", "--", "--check"]),
        ),
        GateCommand::Clippy => process::run(process::cargo(process::PINNED, &root).args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ])),
        GateCommand::Test => {
            process::run(process::cargo(process::PINNED, &root).args([
                "test",
                "--workspace",
                "--all-targets",
                "--locked",
            ]))?;
            process::run(process::cargo(process::PINNED, &root).args([
                "test",
                "--workspace",
                "--doc",
                "--locked",
            ]))
        }
        GateCommand::Lint => lint_all(&root),
        GateCommand::Msrv => {
            process::run(process::cargo(process::MSRV, &root).args([
                "check",
                "--locked",
                "--workspace",
                "--all-targets",
            ]))?;
            for package in ["uarch-tiers", "uarch-core", "uarch-arch"] {
                if root
                    .join("crates")
                    .join(package)
                    .join("Cargo.toml")
                    .exists()
                {
                    process::run(
                        process::cargo(process::MSRV, &root)
                            .args(["test", "--locked", "-p", package]),
                    )?;
                }
            }
            Ok(())
        }
        GateCommand::Ui => {
            process::run(process::cargo(process::PINNED, &root).args([
                "build",
                "--locked",
                "--workspace",
                "--all-targets",
            ]))?;
            process::run(
                process::cargo(process::PINNED, &root)
                    .args(["test", "--locked", "-p", "uarch-macros"])
                    .env("RUST_TEST_THREADS", "1"),
            )
        }
        GateCommand::IrCheck => codegen::ir(&root),
        GateCommand::SizeReport => codegen::size(&root),
        GateCommand::Verify => {
            lint_all(&root)?;
            for arguments in [
                vec!["test", "--workspace", "--all-targets", "--locked"],
                vec![
                    "test",
                    "--workspace",
                    "--all-targets",
                    "--release",
                    "--locked",
                ],
                vec!["test", "--workspace", "--doc", "--locked"],
            ] {
                process::run(process::cargo(process::PINNED, &root).args(arguments))?;
            }
            process::run(process::cargo(process::MSRV, &root).args([
                "check",
                "--workspace",
                "--all-targets",
                "--locked",
            ]))?;
            codegen::abi(&root)?;
            codegen::ir(&root)?;
            codegen::asm(&root)?;
            codegen::mca(&root, None)?;
            codegen::size(&root)
        }
        GateCommand::AbiCheck => codegen::abi(&root),
        GateCommand::AsmCheck => codegen::asm(&root),
        GateCommand::McaGate { cpu } => codegen::mca(&root, cpu.as_deref()),
        GateCommand::Gem5Gate { cpu } => hardware::gem5(&root, cpu.as_deref()),
        GateCommand::Gem5Probe { probe, iterations } => {
            hardware::gem5_probe(probe, iterations.get())
        }
        GateCommand::PmuGate {
            executable,
            operations,
            probe,
            iterations,
            args,
        } => match (executable, operations, probe, iterations) {
            (Some(executable), Some(operations), None, None) => {
                hardware::pmu(&executable, operations.get(), &args)
            }
            (None, None, Some(probe), Some(iterations)) => {
                hardware::pmu_probe(&root, probe, iterations.get())
            }
            (None, None, None, None) => {
                hardware::pmu_probe(&root, hardware::Probe::FastDivisorU64Rem, 200000)
            }
            _ => Err(Error::Gate(
                "select executable/operations or probe/iterations".into(),
            )),
        },
        GateCommand::ExecuteProbe { probe, iterations } => {
            hardware::run_probe(probe, iterations.get())
        }
        GateCommand::Miri => hardware::miri(&root),
        GateCommand::Bench => process::run(process::cargo(process::PINNED, &root).args([
            "bench",
            "--locked",
            "--workspace",
        ])),
    }
}

fn lint_all(root: &std::path::Path) -> Result<()> {
    process::run(process::cargo(process::PINNED, root).args(["fmt", "--all", "--", "--check"]))?;
    process::run(process::cargo(process::PINNED, root).args([
        "clippy",
        "--locked",
        "--workspace",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ]))?;
    lint::check_workspace(root)?;
    lint::check_aarch64(root)?;
    process::run(process::cargo(process::PINNED, root).args([
        "test",
        "--workspace",
        "--test",
        "*scenario*",
        "--release",
        "--locked",
    ]))?;

    process::run(process::cargo(process::PINNED, root).args([
        "clippy",
        "--locked",
        "--workspace",
        "--all-targets",
        "--no-default-features",
        "--",
        "-D",
        "warnings",
    ]))?;
    process::run(process::cargo(process::PINNED, root).args([
        "clippy",
        "--locked",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ]))?;
    for features in ["default", "none", "all"] {
        let mut command = process::cargo(process::PINNED, root);
        command.args(["doc", "--no-deps", "--locked", "--workspace"]);
        if features == "none" {
            command.arg("--no-default-features");
        }
        if features == "all" {
            command.arg("--all-features");
        }
        process::run(command.env("RUSTDOCFLAGS", "-D warnings"))?;
    }
    Ok(())
}
