
use std::{io, fmt, path::Path, process::{self, Command}};
use colored::*;

// Define our error types. These may be customized for our error handling cases.
// Now we will be able to write our own errors, defer to an underlying error
// implementation, or do something in between.
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Process(ProcessError),
}

#[derive(Debug)]
pub struct ProcessError {
    output: process::Output,
    command: Vec<String>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Self::Io(e) => e.fmt(f),
            Self::Process(e) => {
                write!(
                    f,
                    "{} {}\n{} {:?}\n{} {}\n{} {}\n",
                    "process exited with exit code".red(),
                    e.output.status.to_string().red().bold(),
                    "Command:".bold(),
                    e.command,
                    "Stdout:".bold(),
                    String::from_utf8_lossy(&e.output.stdout),
                    "Stderr:".bold(),
                    String::from_utf8_lossy(&e.output.stderr),
                )
            }
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::error::Error for Error {
}

/// Run a git command with the given arguments in the current directory.
pub fn git_cwd(args: &[&str]) -> Result<process::Output, Error> {
    git_internal(args, None)
}

/// Run a git command with the given arguments in the given directory.
pub fn git(args: &[&str], working_dir: &Path) -> Result<process::Output, Error> {
    git_internal(args, Some(working_dir))
}

pub fn git_internal(args: &[&str], working_dir: Option<&Path>) -> Result<process::Output, Error> {
    // eprintln!("{} $ {} {}", working_dir.unwrap_or(Path::new("")).to_string_lossy(), "git".bold(), args.join(" ").bold());

    let mut command = Command::new("git");
    if let Some(working_dir) = working_dir {
        command.current_dir(working_dir);
    }

    let output = command
        .args(args)
        .output()?;

    if !output.status.success() {
        return Err(Error::Process(ProcessError {
            output,
            command: std::iter::once(&"git").chain(args.iter()).map(|&s| s.to_owned()).collect(),
        }));
    }

    Ok(output)
}
