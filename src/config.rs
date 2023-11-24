use std::path::Path;

use anyhow::Result;
use git_commands::git;

use crate::trim::TrimAsciiWhitespace;

/// Get the default branch name from Git config's `init.defaultBranch` setting,
/// falling back to 'master' if it isn't set. This should help handle default
/// branch names that are more awoke.
pub fn default_branch_name(for_path: &Path) -> Result<String> {
    let output = git(
        &[
            "--no-pager",
            "config",
            "--default",
            "master",
            "--get",
            "init.defaultBranch",
        ],
        for_path,
    )?
    .stdout;
    let output = std::str::from_utf8(output.trim_ascii_whitespace())?;
    Ok(output.to_owned())
}
