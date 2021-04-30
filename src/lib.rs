use std::path::{Path, PathBuf};
use anyhow::Result;
use colored::Colorize;
use git_commands::*;

mod conflicts;
use conflicts::*;

pub fn autorebase(repo_path: &Path, onto_branch: &str) -> Result<()> {
    let conflicts_path = repo_path.join(".git/autorebase/conflicts.toml");

    let mut conflicts = if conflicts_path.is_file() {
        Conflicts::read_from_file(&conflicts_path)?
    } else {
        Default::default()
    };

    let worktree_path = repo_path.join(".git/autorebase/autorebase_worktree");

    if !worktree_path.is_dir() {
        create_scratch_worktree(&repo_path, &worktree_path)?;
    }

    // For each branch, find the common ancestor with `master`. There must only be one.

    // TODO: Figure out the entire tree structure.
    // Hmm for now I'll just do it the dumb way.

    let branches = get_eligible_branches(&repo_path)?;
    let current_branch = get_current_branch(&repo_path)?;

    eprintln!("\nCurrent branch: {}", current_branch);
    eprintln!("Branches: {:?}\n", branches);

    for branch in branches.iter() {

        let branch_commit = run_git_cmd_output(&["rev-parse", branch], repo_path)?;
        let branch_commit = String::from_utf8(branch_commit)?;

        // If the rebase for this branch got stopped by a conflict before and
        // it's still the same commit then skip it.
        if conflicts.branches.get(branch) == Some(&branch_commit) {
            eprintln!("Skipping branch {} because it had conflicts last time we tried; rebase manually", branch.yellow());
            continue;
        }

        conflicts.branches.remove(branch);
        conflicts.write_to_file(&conflicts_path)?;

        if branch == onto_branch {
            eprintln!("Skipping branch {} because it is the target", branch.green());
            continue;
        }
        // You can't check out a branch in more than one worktree at a time so
        // if one is already checked out in the main worktree, skip it.
        // This does assume there are no other worktrees. Maybe we should detect
        // if the branch is checked out anywhere directly instead in the same
        // way that Git does it.
        if *branch == current_branch {
            eprintln!("Skipping branch {} because it is checked out", branch.green());
            continue;
        }

        eprintln!("\nRebasing {}\n", branch.bright_green());

        // Get the list of commits we will try to rebase onto (starting with `onto_branch`).
        let target_commit_list = get_target_commit_list(&repo_path, branch, onto_branch)?;

        // Check out the branch.
        checkout_branch(branch, &worktree_path)?;

        let mut stopped_by_conflicts = false;

        for target_commit in target_commit_list {
            eprintln!("\nRebasing onto {}\n", target_commit.bright_green());

            let result = attempt_rebase(&repo_path, &worktree_path, &target_commit)?;
            match result {
                RebaseResult::Success => {
                    eprintln!("\nRebasing onto {}: success\n", target_commit.bright_green());
                    break;
                }
                RebaseResult::Conflict => {
                    eprintln!("\nRebasing onto {}: conflict\n", target_commit.yellow());
                    stopped_by_conflicts = true;
                    continue;
                }
            }
        }

        // Detach HEAD so that the branch can be checked out again in the main worktree.
        run_git_cmd(&["checkout", "--detach"], &worktree_path)?;

        if stopped_by_conflicts {
            // Get the commit again because it will have changed (probably).
            let new_branch_commit = run_git_cmd_output(&["rev-parse", branch], repo_path)?;
            let new_branch_commit = String::from_utf8(new_branch_commit)?;

            conflicts.branches.insert(branch.clone(), new_branch_commit);
            conflicts.write_to_file(&conflicts_path)?;
        }
    }

    Ok(())
}

/// Utility function to get the repo dir for the current directory.
pub fn get_repo_path() -> Result<PathBuf> {
    let output = run_git_cmd_output_cwd(&["rev-parse", "--show-toplevel"])?;
    Ok(PathBuf::from(String::from_utf8(output)?))
}

fn create_scratch_worktree(repo_path: &Path, worktree_path: &Path) -> Result<()> {
    run_git_cmd(&["worktree", "add", "--detach", worktree_path.to_str().unwrap()], repo_path)?; // TODO: Don't unwrap.
    Ok(())
}


// Get the current branch name. Returns `HEAD` if detached.
fn get_current_branch(repo_path: &Path) -> Result<String> {
    // --symbolic-full-name makes this work even if there is a branch/tag named `HEAD`.
    // Except it still prints `HEAD` as the output so :shrug:.
    let output = run_git_cmd_output(&["rev-parse", "--symbolic-full-name", "--abbrev-ref", "HEAD"], repo_path)?;
    Ok(String::from_utf8(output)?)
}

fn get_eligible_branches(repo_path: &Path) -> Result<Vec<String>> {

    // TODO: Config system to allow specifying the branches? Maybe allow adding/removing them?
    // Store config in `.git/autorebase/autorebase.toml` or `autorebase.toml`?

    let output = run_git_cmd_output(&["for-each-ref", "--format=%(refname:short)", "refs/heads"], repo_path)?;
    let output = String::from_utf8(output)?;
    Ok(output.lines().map(ToOwned::to_owned).collect())
}

fn get_merge_base(repo_path: &Path, a: &str, b: &str) -> Result<String> {
    let output = run_git_cmd_output(&["merge-base", a, b], repo_path)?;
    let output = String::from_utf8(output)?;
    // TODO: Could be very slightly more efficient if we trim whitespace from the Vec<u8> instead.
    Ok(output.trim().to_owned())
}

fn checkout_branch(branch: &str, repo_path: &Path) -> Result<()> {
    run_git_cmd(&["switch", branch], repo_path)?;
    Ok(())
}

fn is_rebasing(repo_path: &Path, worktree: Option<&str>) -> bool {
    // Check `.git/rebase-merge` exists. See https://stackoverflow.com/questions/3921409/how-to-know-if-there-is-a-git-rebase-in-progress/67245016#67245016

    let worktree_git_dir = if let Some(worktree) = worktree {
        repo_path.join(".git/worktrees").join(worktree)
    } else {
        repo_path.join(".git")
    };

    let rebase_apply = worktree_git_dir.join("rebase-apply");
    let rebase_merge = worktree_git_dir.join("rebase-merge");

    rebase_apply.exists() || rebase_merge.exists()
}

enum RebaseResult {
    Success,
    Conflict,
}

fn attempt_rebase(repo_path: &Path, worktree_path: &Path, onto: &str) -> Result<RebaseResult> {
    let rebase_ok = run_git_cmd(&["rebase", onto], worktree_path);
    if rebase_ok.is_ok() {
        return Ok(RebaseResult::Success)
    }

    // We may need to abort if the rebase is still in progress. Git checks
    // the rebase status like this:
    // https://stackoverflow.com/questions/3921409/how-to-know-if-there-is-a-git-rebase-in-progress/67245016#67245016

    if is_rebasing(repo_path, Some("autorebase_worktree")) {
        // Abort the rebase.
        run_git_cmd(&["rebase", "--abort"], worktree_path)?;
    }

    Ok(RebaseResult::Conflict)
}

fn get_target_commit_list(repo_path: &Path, branch: &str, onto: &str) -> Result<Vec<String>> {
    let merge_base = get_merge_base(repo_path, branch, onto)?;

    let output = run_git_cmd_output(&["log", "--format=%H", &format!("{}..{}", merge_base, onto)], repo_path)?;
    let output = String::from_utf8(output)?;
    Ok(output.lines().map(ToOwned::to_owned).collect())
}
