
use std::{path::Path, process::{Command, Stdio}, io};
use anyhow::{bail, Result};
use io::Write;
use colored::Colorize;

pub fn run_git_cmd_output_cwd(args: &[&str]) -> Result<Vec<u8>> {
    eprintln!("~ {} {}", "git".bold(), args.join(" ").bold());

    let output = Command::new("git")
        .args(args)
        .stderr(Stdio::inherit()) // Print stderr to console.
        .output()?;

    // Print stdout to console.
    io::stdout().write_all(&output.stdout)?;

    if !output.status.success() {
        bail!("Command failed"); // TODO: Better error.
    }

    Ok(output.stdout)
}

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
            "Command failed\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ); // TODO: Better error.
    }

    Ok(output.stdout)
}
