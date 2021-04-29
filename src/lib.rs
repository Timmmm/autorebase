use std::path::{Path, PathBuf};
use anyhow::Result;
use colored::Colorize;

use git_commands::*;

pub fn autorebase(repo_dir: &Path, target_branch: &str) -> Result<()> {
    let worktree_dir = repo_dir.join(".git/autorebase/autorebase_worktree");

    if !worktree_dir.is_dir() {
        create_scratch_worktree(&repo_dir, &worktree_dir)?;
    }

    // For each branch, find the common ancestor with `master`. There must only be one.

    // TODO: Figure out the entire tree structure.
    // Hmm for now I'll just do it the dumb way.

    let branches = get_eligible_branches(&repo_dir)?;
    let current_branch = get_current_branch(&repo_dir)?;

    eprintln!("\nCurrent branch: {}", current_branch);
    eprintln!("Branches: {:?}\n", branches);

    for branch in branches.iter() {
        if branch == target_branch {
            eprintln!("Skipping branch {} because it is the target", branch.green());
            continue;
        }
        if *branch == current_branch {
            eprintln!("Skipping branch {} because it is checked out", branch.green());
            continue;
        }

        eprintln!("\nRebasing {}\n", branch.bright_green());
        // Do a binary search of attempted rebases.

        // git merge-base HEAD master
        // git log <merge-base>..master

        // Check out the branch.
        checkout_branch(branch, &worktree_dir)?;
        let _merge_base = get_merge_base(&worktree_dir, "HEAD", target_branch)?;
        attempt_rebase(&repo_dir, &worktree_dir, target_branch)?;

        // TODO: Handle branches with more than one branch label on them.

        // TODO: Store info about whether or not we were able to rebase branches, so
        // we don't keep trying to rebase branches that can go no further.
    }

    Ok(())
}

/// Utility function to get the repo dir for the current directory.
pub fn get_repo_dir() -> Result<PathBuf> {
    let output = run_git_cmd_output_cwd(&["rev-parse", "--show-toplevel"])?;
    Ok(PathBuf::from(String::from_utf8(output)?))
}

fn create_scratch_worktree(repo_dir: &Path, worktree_dir: &Path) -> Result<()> {
    run_git_cmd(&["worktree", "add", "--detach", worktree_dir.to_str().unwrap()], repo_dir)?; // TODO: Don't unwrap.
    Ok(())
}


// Get the current branch name. Returns `HEAD` if detached.
fn get_current_branch(repo_dir: &Path) -> Result<String> {
    // --symbolic-full-name makes this work even if there is a branch/tag named `HEAD`.
    // Except it still prints `HEAD` as the output so :shrug:.
    let output = run_git_cmd_output(&["rev-parse", "--symbolic-full-name", "--abbrev-ref", "HEAD"], repo_dir)?;
    Ok(String::from_utf8(output)?)
}

fn get_eligible_branches(repo_dir: &Path) -> Result<Vec<String>> {
    let output = run_git_cmd_output(&["for-each-ref", "--format=%(refname:short)", "refs/heads"], repo_dir)?;
    let output = String::from_utf8(output)?;
    Ok(output.lines().map(ToOwned::to_owned).collect())
}

fn get_merge_base(repo_dir: &Path, a: &str, b: &str) -> Result<String> {
    let output = run_git_cmd_output(&["merge-base", a, b], repo_dir)?;
    Ok(String::from_utf8(output)?)
}

fn checkout_branch(branch: &str, repo_dir: &Path) -> Result<()> {
    run_git_cmd(&["switch", branch], repo_dir)?;
    Ok(())
}

fn is_rebasing(repo_dir: &Path, worktree: Option<&str>) -> bool {
    // Check `.git/rebase-merge` exists. See https://stackoverflow.com/questions/3921409/how-to-know-if-there-is-a-git-rebase-in-progress/67245016#67245016

    let worktree_git_dir = if let Some(worktree) = worktree {
        repo_dir.join(".git/worktrees").join(worktree)
    } else {
        repo_dir.join(".git")
    };

    let rebase_apply = worktree_git_dir.join("rebase-apply");
    let rebase_merge = worktree_git_dir.join("rebase-merge");

    rebase_apply.exists() || rebase_merge.exists()
}

fn attempt_rebase(repo_dir: &Path, worktree_dir: &Path, target_branch: &str) -> Result<()> {
    let rebase_ok = run_git_cmd(&["rebase", target_branch], worktree_dir);
    if rebase_ok.is_ok() {
        return Ok(())
    }

    // We may need to abort if the rebase is still in progress. Git checks
    // the rebase status like this:
    // https://stackoverflow.com/questions/3921409/how-to-know-if-there-is-a-git-rebase-in-progress/67245016#67245016

    if is_rebasing(repo_dir, Some("autorebase_worktree")) {
        // Abort the rebase.
        run_git_cmd(&["rebase", "--abort"], repo_dir)?;
        // TODO - try rebasing one commit at a time going backwards from
        // master until we get to the merge point.

        // let commit_list = run_git_cmd(&["rev-list", format!("{}..{}", merge_base, target_branch), repo_dir)?;
    }

    Ok(())
}
