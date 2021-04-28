
use std::{path::Path, process::{Command, Stdio}, io};
use anyhow::{bail, Result};
use io::Write;
use colored::Colorize;

/// Run a git command with the given arguments in the current directory.
/// Return the standard output. The command and stdout/err are printed to the console.
pub fn run_git_cmd_output_cwd(args: &[&str]) -> Result<Vec<u8>> {
    eprintln!("~ {} {}", "git".bold(), args.join(" ").bold());

    let output = Command::new("git")
        .args(args)
        .stderr(Stdio::inherit()) // Print stderr to console.
        .output()?;

    // Print stdout to console.
    io::stdout().write_all(&output.stdout)?;

    if !output.status.success() {
        bail!("Command failed with exit code {}", output.status);
    }

    Ok(output.stdout)
}

/// Run a git command with the given arguments in the given directory.
/// The command and stdout/err are printed to the console.
pub fn run_git_cmd(args: &[&str], working_dir: &Path) -> Result<()> {
    eprintln!("~ {} {}", "git".bold(), args.join(" ").bold());

    let status = Command::new("git")
        .current_dir(working_dir)
        .args(args)
        .status()?;

    if !status.success() {
        bail!("Command failed with exit code: {}", status);
    }

    Ok(())
}

/// Run a git command with the given arguments in the given directory.
/// Return the standard output. The command and stdout/err are printed to the console.
pub fn run_git_cmd_output(args: &[&str], working_dir: &Path) -> Result<Vec<u8>> {
    eprintln!("~ {} {}", "git".bold(), args.join(" ").bold());

    let output = Command::new("git")
        .current_dir(working_dir)
        .args(args)
        .stderr(Stdio::inherit()) // Print stderr to console.
        .output()?;

    // Print stdout to console.
    io::stdout().write_all(&output.stdout)?;

    if !output.status.success() {
        bail!(
            "Command failed with exit code {}\nStdout:{}\nStderr:{}\n",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    Ok(output.stdout)
}
