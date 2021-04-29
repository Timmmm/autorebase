// Tool to automatically rebase branches.

use argh::FromArgs;
use anyhow::Result;

use autorebase::{get_repo_path, autorebase};

#[derive(FromArgs)]
/// Automatically rebase some branches.
struct CliOptions {
    /// the target branch (typically "master" or "develop")
    #[argh(option, default="String::from(\"master\")")]
    target_branch: String,
}

fn main() -> Result<()> {
    let options: CliOptions = argh::from_env();

    // Find the repo dir in the same way git does.
    let repo_path = get_repo_path()?;

    autorebase(&repo_path, &options.target_branch)?;

    Ok(())
}
