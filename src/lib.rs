use anyhow::{anyhow, bail, Result};
use colored::*;
use git_commands::*;
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

mod conflicts;
use conflicts::*;
mod trim;
use trim::*;

// Set GIT_COMMITTER_DATE to now to prevent getting inconsistent hashes when
// rebasing the same commit multiple times.
fn set_committer_date_to_now() {
    // Only set it if it isn't already set, otherwise it breaks test and also
    // the user might want to set it.
    if env::var_os("GIT_COMMITTER_DATE").is_some() {
        return;
    }

    let time_since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    env::set_var(
        "GIT_COMMITTER_DATE",
        format!("@{} +0000", time_since_epoch.as_secs()),
    );
}

/// Autorebase all branches in `repo_path` onto the `onto_branch` (typically "master").
/// If `slow_conflict_detection` is true it will try every commit when there is a
/// conflict until one works. Reliably but slow. If it is false it will try to
/// detect the first commit that causes a conflict and rebase to just before that.
/// Way faster, but may not always work.
///
pub fn autorebase(
    repo_path: &Path,
    onto_branch: &str,
    slow_conflict_detection: bool,
) -> Result<()> {
    // Check the git version. `git switch` was introduced in 2.23.
    if git_version(repo_path)?.as_slice() < &[2, 23] {
        bail!("Your Git installation is too old - version 2.23 or later is required");
    }

    // The first thing we do is set the commiter date to now. If we don't do this
    // then when we have two branch labels on the same commit, when they get
    // rebased they will be given different commiter dates which will mean they
    // get different hashes and end up as separate commits.
    set_committer_date_to_now();

    let conflicts_path = repo_path.join(".git/autorebase/conflicts.toml");

    let mut conflicts = if conflicts_path.is_file() {
        Conflicts::read_from_file(&conflicts_path)?
    } else {
        Default::default()
    };

    let worktree_path = repo_path.join(".git/autorebase/autorebase_worktree");

    if !worktree_path.is_dir() {
        eprint!("{}", "• Creating worktree...".yellow());
        create_scratch_worktree(&repo_path, &worktree_path)?;
        eprintln!("\r{}", "• Creating worktree...".green());
    }

    // For each branch, find the common ancestor with `master`. There must only be one.

    eprint!("{}", "• Getting branches...".yellow());
    let all_branches = get_branches(&repo_path)?;
    let onto_branch_info = all_branches
        .iter()
        .find(|b| b.branch == onto_branch)
        .ok_or_else(|| anyhow!("Couldn't find target branch '{}'", onto_branch))?;
    eprintln!("\r{}", "• Getting branches...".green());

    // Print a summary of the branches.
    for branch in all_branches.iter() {
        if branch.branch == onto_branch {
            eprintln!("    - {} (target branch)", branch.branch.blue().bold());
        } else if branch.upstream.is_some() {
            eprintln!(
                "    - {} (skipping because it has an upstream)",
                branch.branch.bold()
            );
        } else if matches!(&branch.worktree, Some(worktree) if !worktree.clean) {
            eprintln!(
                "    - {} (skipping because it is checked out and not clean)",
                branch.branch.bold()
            );
        } else {
            eprintln!("    - {}", branch.branch.green().bold());
        }
    }

    // Get the branches that we will actually attempt to rebase.
    let rebase_branches: Vec<&BranchInfo> = all_branches
        .iter()
        .filter(|branch| {
            branch.branch != onto_branch
                && branch.upstream.is_none()
                && !matches!(&branch.worktree, Some(worktree) if !worktree.clean)
        })
        .collect();

    // Pull master.
    pull_master(onto_branch_info, &worktree_path)?;

    for branch in rebase_branches.iter() {
        rebase_branch(
            branch,
            repo_path,
            &mut conflicts,
            &conflicts_path,
            onto_branch,
            &worktree_path,
            slow_conflict_detection,
        )?;
    }

    Ok(())
}

/// Pull the master branch (the `onto` branch), if it has an upstream.
fn pull_master(onto_branch_info: &BranchInfo, worktree_path: &Path) -> Result<(), anyhow::Error> {
    if onto_branch_info.upstream.is_some() {
        if let Some(onto_branch_worktree_info) = &onto_branch_info.worktree {
            // It's checked out somewhere. Check if that worktree is clean,
            // if so pull it there.
            if onto_branch_worktree_info.clean {
                eprint!(
                    "{} {}{}",
                    "• Pulling".yellow(),
                    onto_branch_info.branch.yellow().bold(),
                    "...".yellow(),
                );

                git(&["pull", "--ff-only"], &onto_branch_worktree_info.path)?;

                eprintln!(
                    "\r{} {}{}",
                    "• Pulling".green(),
                    onto_branch_info.branch.green().bold(),
                    "...".green(),
                );
            } else {
                eprintln!(
                    "• Not pulling target branch {} because it is checked out and has pending changes",
                    onto_branch_info.branch.bold(),
                );
            }
        } else {
            eprint!(
                "{} {}{}",
                "• Pulling".yellow(),
                onto_branch_info.branch.yellow().bold(),
                "...".yellow(),
            );

            git(&["switch", &onto_branch_info.branch], worktree_path)?;
            git(&["pull", "--ff-only"], worktree_path)?;
            git(&["switch", "--detach"], worktree_path)?;

            eprintln!(
                "\r{} {}{}",
                "• Pulling".green(),
                onto_branch_info.branch.green().bold(),
                "...".green(),
            );
        }
    } else {
        eprintln!(
            "{} {} {}",
            "• Warning: Not pulling target branch".yellow(),
            onto_branch_info.branch.yellow().bold(),
            "because it has no upstream".yellow(),
        );
    }
    Ok(())
}

fn rebase_branch(
    branch: &BranchInfo,
    repo_path: &Path,
    conflicts: &mut Conflicts,
    conflicts_path: &Path,
    onto_branch: &str,
    worktree_path: &Path,
    slow_conflict_detection: bool,
) -> Result<(), anyhow::Error> {
    eprintln!("• Rebasing {}...", branch.branch.bold());

    let branch_commit = get_commit_hash(repo_path, &branch.branch)?;

    if conflicts.branches.get(&branch.branch).map(|s| s.as_str()) == Some(&branch_commit) {
        eprintln!(
            "{}",
            "    - Skipping rebase because it had conflicts last time we tried; rebase manually"
                .yellow()
        );
        return Ok(());
    }

    conflicts.branches.remove(&branch.branch);
    conflicts.write_to_file(&conflicts_path)?;

    let merge_base = get_merge_base(repo_path, &branch.branch, onto_branch)?;

    let target_commit_list = get_commit_list(repo_path, &merge_base, onto_branch)?;

    if target_commit_list.is_empty() {
        eprintln!("    - No rebase necessary");
        return Ok(());
    }

    // The worktree we will use for the rebase. If it is already checked out
    // in a worktree somewhere, use that one. Otherwise use our temporary one.
    let rebase_worktree_path = if let Some(worktree) = &branch.worktree {
        // It's checked out in a worktree somewhere.
        &worktree.path
    } else {
        // It isn't checked out anywhere; switch to it in our temporary worktree.
        switch_to_branch(&branch.branch, &worktree_path)?;
        worktree_path
    };

    let mut stopped_by_conflicts = false;

    if slow_conflict_detection {
        for target_commit in target_commit_list {
            eprintln!("    - Rebasing onto {}", target_commit.bold());

            let result = attempt_rebase(&repo_path, rebase_worktree_path, &target_commit)?;
            match result {
                RebaseResult::Success => {
                    eprintln!("{}", "    - Success!".green());
                    break;
                }
                RebaseResult::Conflict => {
                    eprintln!("{}", "    - Conflicts...".yellow());
                    stopped_by_conflicts = true;
                    continue;
                }
            }
        }
    } else {
        let result = attempt_rebase(&repo_path, rebase_worktree_path, &target_commit_list[0])?;
        match result {
            RebaseResult::Success => {
                eprintln!("{}", "    - Success!".green());
            }
            RebaseResult::Conflict => {
                eprintln!("{}", "    - Conflicts...".yellow());
                stopped_by_conflicts = true;

                eprintln!("    - Finding first conflict...");

                // Save the current checkout state.
                let old_location = get_current_branch_or_commit(rebase_worktree_path)?;

                let num_nonconflicting_commits = count_nonconflicting_commits_via_rebase(
                    repo_path,
                    rebase_worktree_path,
                    &branch.branch,
                    onto_branch,
                )?;

                // Restore the previous state.
                switch_to_branch_or_commit(rebase_worktree_path, &old_location)?;

                if num_nonconflicting_commits > 0
                    && num_nonconflicting_commits < target_commit_list.len()
                {
                    let last_nonconflicting_commit =
                        &target_commit_list[target_commit_list.len() - num_nonconflicting_commits];

                    // Make a temporary branch, then try to rebase master onto it.
                    // Then see which commit failed. Finally try to rebase
                    // the branch onto master at the last commit that succeeded.

                    let result = attempt_rebase(
                        &repo_path,
                        rebase_worktree_path,
                        &last_nonconflicting_commit,
                    )?;
                    match result {
                        RebaseResult::Success => {
                            eprintln!("{}", "    - Success!".green());
                        }
                        RebaseResult::Conflict => {
                            eprintln!("{}", "    - Conflicts...".yellow());
                        }
                    }
                }
            }
        }
    }

    // Switch to the branch so that we don't leave references to unneeded commits
    // around, and detach otherwise we may prevent people checking it out.
    git(&["switch", "--detach", &branch.branch], &worktree_path)?;

    if stopped_by_conflicts {
        eprintln!(
            "{}",
            "    - Rebase stunted by conflicts. Rebase manually.".yellow()
        );

        // Get the commit again because it will have changed (probably).
        let new_branch_commit = get_commit_hash(repo_path, &branch.branch)?;

        conflicts
            .branches
            .insert(branch.branch.clone(), new_branch_commit);
        conflicts.write_to_file(&conflicts_path)?;
    }

    Ok(())
}

/// Utility function to get the repo dir for the current directory.
pub fn get_repo_path() -> Result<PathBuf> {
    let output = git_cwd(&["rev-parse", "--show-toplevel"])?.stdout;
    let output = std::str::from_utf8(output.trim_ascii_whitespace())?;
    Ok(PathBuf::from(output))
}

fn create_scratch_worktree(repo_path: &Path, worktree_path: &Path) -> Result<()> {
    let worktree_path = worktree_path
        .to_str()
        .ok_or_else(|| anyhow!("worktree path is not unicode"))?;
    git(&["worktree", "add", "--detach", worktree_path], repo_path)?;
    Ok(())
}

#[derive(Debug)]
struct WorktreeInfo {
    // Path to the worktree.
    path: PathBuf,
    // Are both the index (staging area) and worktree clean? There may be untracked files.
    clean: bool,
}

#[derive(Debug)]
struct BranchInfo {
    branch: String,
    upstream: Option<String>,
    worktree: Option<WorktreeInfo>,
}

fn get_branches(repo_path: &Path) -> Result<Vec<BranchInfo>> {
    use std::str;

    // TODO: Config system to allow specifying the branches? Maybe allow adding/removing them?
    // Store config in `.git/autorebase/autorebase.toml` or `autorebase.toml`?

    let output = git(
        &[
            "for-each-ref",
            "--format=%(refname:short)%00%(upstream:short)%00%(worktreepath)",
            "refs/heads",
        ],
        repo_path,
    )?
    .stdout;
    let branches = output
        .split(|c| *c == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            let parts: Vec<&[u8]> = line.split(|c| *c == 0).collect();
            if parts.len() != 3 {
                bail!(
                    "for-each-ref parse error, got {} parts, expected 3",
                    parts.len()
                );
            }

            let branch = str::from_utf8(parts[0])?.to_owned();

            let upstream = if parts[1].is_empty() {
                None
            } else {
                Some(str::from_utf8(parts[1])?.to_owned())
            };

            let worktree = if parts[2].is_empty() {
                None
            } else {
                let path = str::from_utf8(parts[2])?;
                let path = PathBuf::from(path);
                let clean = is_clean(&path);
                Some(WorktreeInfo { path, clean })
            };

            Ok(BranchInfo {
                branch,
                upstream,
                worktree,
            })
        })
        .collect::<Result<_, _>>()?;
    Ok(branches)
}

fn get_merge_base(repo_path: &Path, a: &str, b: &str) -> Result<String> {
    let output = git(&["merge-base", a, b], repo_path)?.stdout;
    let output = std::str::from_utf8(output.trim_ascii_whitespace())?;
    Ok(output.to_owned())
}

fn switch_to_branch(branch: &str, repo_path: &Path) -> Result<()> {
    git(&["switch", branch], repo_path)?;
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

fn is_clean(worktree_path: &Path) -> bool {
    // Run `git diff-index --quiet HEAD` and `git diff-index --quiet --cached HEAD`
    // to check if there are any changes in the working tree or index (staging area).

    // Since this uses the exit code (0 = no differences) we kind of have to ignore
    // other errors since there's no way to detect them.

    git(&["diff-index", "--quiet", "HEAD"], worktree_path).is_ok()
        && git(
            &["diff-index", "--quiet", "--cached", "HEAD"],
            worktree_path,
        )
        .is_ok()
}

enum RebaseResult {
    Success,
    Conflict,
}

fn attempt_rebase(repo_path: &Path, worktree_path: &Path, onto: &str) -> Result<RebaseResult> {
    let rebase_ok = git(&["rebase", onto], worktree_path);
    if rebase_ok.is_ok() {
        return Ok(RebaseResult::Success);
    }

    // We may need to abort if the rebase is still in progress. Git checks
    // the rebase status like this:
    // https://stackoverflow.com/questions/3921409/how-to-know-if-there-is-a-git-rebase-in-progress/67245016#67245016

    if is_rebasing(repo_path, Some("autorebase_worktree")) {
        // Abort the rebase.
        git(&["rebase", "--abort"], worktree_path)?;
    }

    Ok(RebaseResult::Conflict)
}

/// Create a temporary branch at master (`onto`), then try to rebase it ont
/// `branch`. Count how many commits were rebased successfully, and
/// return that number. Then abort the rebase, and delete the branch.
///
/// Note that this will change the checked out branch.
fn count_nonconflicting_commits_via_rebase(
    repo_path: &Path,
    worktree_path: &Path,
    branch: &str,
    onto: &str,
) -> Result<usize> {
    // Create a temporary branch at master. If it already exists (e.g. because
    // a previous command failed) just reset it to here.
    git(
        &[
            "switch",
            "--force-create",
            "autorebase_tmp_safe_to_delete",
            onto,
        ],
        worktree_path,
    )?;

    // Rebase onto branch.
    let rebase_ok = git(&["rebase", branch], worktree_path);
    if rebase_ok.is_ok() {
        // Rebase worked one way but not in the other. Bit weird. This probably
        // shouldn't happen normally but we'll just give up.
        return Ok(0);
    }

    if !is_rebasing(repo_path, Some("autorebase_worktree")) {
        // Error - it should be rebasing!
        bail!("Rebase failed but repo is not rebasing.");
    }

    // Count how many commits we successfully applied.
    let commit_list = get_commit_list(worktree_path, branch, "HEAD")?;

    // Abort the rebase.
    git(&["rebase", "--abort"], worktree_path)?;

    // Delete the branch and checkout master (detached) otherwise we risk
    // keeping commits around.
    git(&["switch", "--detach", onto], worktree_path)?;

    git(
        &[
            "branch",
            "--delete",
            "--force",
            "autorebase_tmp_safe_to_delete",
        ],
        worktree_path,
    )?;

    Ok(commit_list.len())
}

/// Get the list of commits from `from` to `to`. The list includes `to` but not
/// `from`.
fn get_commit_list(repo_path: &Path, from: &str, to: &str) -> Result<Vec<String>> {
    let output = git(
        &[
            "--no-pager",
            "log",
            "--format=%H",
            &format!("{}..{}", from, to),
        ],
        repo_path,
    )?
    .stdout;
    let output = String::from_utf8(output)?;
    Ok(output.lines().map(ToOwned::to_owned).collect())
}

/// Return the Git version like [2, 3, 30]. Really annoyingly the version sometimes
/// includes text, for example 2.31.1.windows.1 (yes really). We will just convert
/// unparsable values to -1. Ugly but they started it.
fn git_version(repo_path: &Path) -> Result<Vec<i32>> {
    // The output of `git version` is guaranteed to be stable, though it has a stupid
    // "git version " string at the start.
    let output = git(&["version"], repo_path)?.stdout;
    let output = std::str::from_utf8(output.trim_ascii_whitespace())?;

    if let Some(version_string) = output.strip_prefix("git version ") {
        Ok(version_string
            .split('.')
            .map(|s| s.parse().unwrap_or(-1))
            .collect())
    } else {
        bail!("Invalid `git version` output");
    }
}

enum BranchOrCommit {
    Branch(String),
    Commit(String),
}

/// Get the current branch, if we are on one.
///
/// `git symbolic-ref --quiet --short HEAD` returns the branch name, or returns
/// an error if we are not on a branch. Note it will still return the branch
/// name if we are on an unborn branch.
///
fn get_current_branch(worktree_path: &Path) -> Result<Option<String>> {
    let output = Command::new("git")
        .current_dir(worktree_path)
        .args(&["symbolic-ref", "--quiet", "--short", "HEAD"])
        .output()?;

    if output.status.success() {
        let branch = std::str::from_utf8(output.stdout.trim_ascii_whitespace())?;
        Ok(Some(branch.to_owned()))
    } else {
        Ok(None)
    }
}

/// Use `git rev-parse HEAD` to return the current commit hash. It will
/// return an error if we are on an unborn branch. That's an error for us though
/// so we don't have to treat that case specially.
fn get_commit_hash(worktree_path: &Path, branch: &str) -> Result<String> {
    let commit = git(&["rev-parse", &branch], worktree_path)?.stdout;
    let commit = std::str::from_utf8(commit.trim_ascii_whitespace())?;
    Ok(commit.to_owned())
}

fn get_current_branch_or_commit(worktree_path: &Path) -> Result<BranchOrCommit> {
    if let Some(branch) = get_current_branch(worktree_path)? {
        return Ok(BranchOrCommit::Branch(branch));
    }

    let commit = get_commit_hash(worktree_path, "HEAD")?;
    Ok(BranchOrCommit::Commit(commit))
}

fn switch_to_branch_or_commit(
    worktree_path: &Path,
    branch_or_commit: &BranchOrCommit,
) -> Result<()> {
    match branch_or_commit {
        BranchOrCommit::Branch(ref branch) => {
            git(&["switch", &branch], worktree_path)?;
        }
        BranchOrCommit::Commit(ref commit) => {
            git(&["switch", "--detach", &commit], worktree_path)?;
        }
    }
    Ok(())
}
