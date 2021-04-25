// Tool to automatically rebase branches.

use argh::FromArgs;
use std::path::{Path, PathBuf};
use anyhow::Result;
use colored::Colorize;

mod git_commands;
use git_commands::*;

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
    let repo_dir = get_repo_dir()?;

    autorebase(&repo_dir, &options.target_branch)?;

    Ok(())
}

fn autorebase(repo_dir: &Path, target_branch: &str) -> Result<()> {
    let worktree_dir = repo_dir.join(".git/autorebase");

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


fn get_repo_dir() -> Result<PathBuf> {
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

fn attempt_rebase(repo_dir: &Path, worktree_dir: &Path, target_branch: &str) -> Result<()> {
    let rebase_ok = run_git_cmd(&["rebase", target_branch], worktree_dir);
    if rebase_ok.is_ok() {
        return Ok(())
    }

    // We may need to abort if the rebase is still in progress. Git checks
    // the rebase status like this:
    // https://stackoverflow.com/questions/3921409/how-to-know-if-there-is-a-git-rebase-in-progress/67245016#67245016

    let rebase_apply = repo_dir.join(".git/worktress/autorebase/rebase-apply");
    let rebase_merge = repo_dir.join(".git/worktress/autorebase/rebase-merge");

    if rebase_apply.exists() || rebase_merge.exists() {
        // Abort the rebase.
        run_git_cmd(&["rebase", "--abort"], repo_dir)?;
        // TODO - try rebasing one commit at a time going backwards from
        // master until we get to the merge point.

        // let commit_list = run_git_cmd(&["rev-list", format!("{}..{}", merge_base, target_branch), repo_dir)?;
    }

    Ok(())
}


#[cfg(test)]
mod tests {
    use tempfile::{TempDir, tempdir};
    use std::fs::write;
    use super::*;
    use std::process::Command;

    fn create_temporary_git_repo() -> TempDir {
        let repo_dir = tempdir().expect("Couldn't create temporary directory");
        run_git_cmd(&["init", "--initial-branch=master"], &repo_dir.path()).expect("error initialising git repo");
        // You have to set these otherwise Git can't do commits.
        run_git_cmd(&["config", "user.email", "me@example.com"], &repo_dir.path()).expect("error setting config");
        run_git_cmd(&["config", "user.name", "Me"], &repo_dir.path()).expect("error setting config");
        repo_dir
    }

    fn commit(message: &str, working_dir: &Path) {
        assert!(
            Command::new("git")
                .current_dir(working_dir)
                .args(&["commit", "-m", message])
                .status()
                .expect("failed to execute git process")
                .success()
        );
    }

    fn print_log(repo_dir: &Path) {
        run_git_cmd(&["--no-pager", "log", "--oneline", "--decorate", "--graph", "--all"], repo_dir).expect("git log failed");
    }

    #[test]
    fn basic_test() {
        let repo = create_temporary_git_repo();
        let repo_dir = repo.into_path();

        eprintln!("Running test in {:?}", &repo_dir);

        write(&repo_dir.join("data1.txt"), "1").expect("unable to write file");
        run_git_cmd(&["add", "data1.txt"], &repo_dir).expect("git command failed");
        commit("Commit 1\n\nMore info", &repo_dir);

        // Checkout a new branch
        run_git_cmd(&["checkout", "-b", "two"], &repo_dir).expect("git command failed");

        write(&repo_dir.join("data2.txt"), "2").expect("unable to write file");
        run_git_cmd(&["add", "data2.txt"], &repo_dir).expect("git command failed");
        commit("Commit 2\n\nMore info", &repo_dir);

        // Go back to master.
        run_git_cmd(&["checkout", "master"], &repo_dir).expect("git command failed");

        write(&repo_dir.join("data3.txt"), "3").expect("unable to write file");
        run_git_cmd(&["add", "data3.txt"], &repo_dir).expect("git command failed");
        commit("Commit 3\n\nMore info", &repo_dir);

        // Log before.
        print_log(&repo_dir);

        // Auto-rebase!
        autorebase(&repo_dir, "master").expect("autorebase failed");

        // Log after.
        print_log(&repo_dir);
    }



}
