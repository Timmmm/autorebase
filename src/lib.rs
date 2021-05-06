use std::path::{Path, PathBuf};
use anyhow::{anyhow, bail, Result};
use git_commands::*;
use colored::*;

mod conflicts;
use conflicts::*;
mod trim;
use trim::*;

pub fn autorebase(repo_path: &Path, onto_branch: &str) -> Result<()> {
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

    // TODO: Figure out the entire tree structure.
    // Hmm for now I'll just do it the dumb way.

    eprint!("{}", "• Getting branches...".yellow());
    let all_branches = get_branches(&repo_path)?;
    let onto_branch_info = all_branches.iter().find(|b| b.branch == onto_branch);
    eprintln!("\r{}", "• Getting branches...".green());

    // Print a summary of the branches.
    for branch in all_branches.iter() {
        if branch.branch == onto_branch {
            eprintln!("    - {} (target branch)", branch.branch.blue().bold());
        } else if branch.upstream.is_some() {
            eprintln!("    - {} (skipping because it has an upstream)", branch.branch.bold());
        } else if matches!(&branch.worktree, Some(worktree) if !worktree.clean) {
            eprintln!("    - {} (skipping because it is checked out and not clean)", branch.branch.bold());
        } else {
            eprintln!("    - {}", branch.branch.green().bold());
        }
    }

    // Get the branches that we will actually attempt to rebase.
    let rebase_branches: Vec<&BranchInfo> = all_branches.iter().filter(|branch| {
        branch.branch != onto_branch && branch.upstream.is_none() && !matches!(&branch.worktree, Some(worktree) if !worktree.clean)
    }).collect();

    // Pull master.
    if let Some(onto_branch_info) = onto_branch_info {
        if onto_branch_info.upstream.is_some() {
            if let Some(onto_branch_worktree_info) = &onto_branch_info.worktree {
                // It's checked out somewhere. Check if that worktree is clean,
                // if so pull it there.
                if onto_branch_worktree_info.clean {
                    eprint!(
                        "{} {}{}",
                        "• Pulling".yellow(),
                        onto_branch.yellow().bold(),
                        "...".yellow(),
                    );

                    git(&["pull", "--ff-only"], &onto_branch_worktree_info.path)?;

                    eprintln!(
                        "\r{} {}{}",
                        "• Pulling".green(),
                        onto_branch.green().bold(),
                        "...".green(),
                    );
                } else {
                    eprintln!(
                        "• Not pulling target branch {} because it is checked out and has pending changes",
                        onto_branch.bold(),
                    );
                }
            } else {
                eprint!(
                    "{} {}{}",
                    "• Pulling".yellow(),
                    onto_branch.yellow().bold(),
                    "...".yellow(),
                );

                git(&["checkout", onto_branch], &worktree_path)?;
                git(&["pull", "--ff-only"], &worktree_path)?;
                git(&["checkout", "--detach"], &worktree_path)?;

                eprintln!(
                    "\r{} {}{}",
                    "• Pulling".green(),
                    onto_branch.green().bold(),
                    "...".green(),
                );
            }
        } else {
            eprintln!(
                "{} {} {}",
                "• Warning: Not pulling target branch".yellow(),
                onto_branch.yellow().bold(),
                "because it has no upstream".yellow(),
            );
        }
    } else {
        eprintln!(
            "{} {} {}",
            "• Warning: Not pulling target branch".yellow(),
            onto_branch.yellow().bold(),
            "because it was not found".yellow(),
        );
    }

    for branch in rebase_branches.iter() {
        eprintln!("• Rebasing {}...", branch.branch.bold());

        let branch_commit = git(&["rev-parse", &branch.branch], repo_path)?.stdout;
        let branch_commit = std::str::from_utf8(branch_commit.trim_ascii_whitespace())?;

        // If the rebase for this branch got stopped by a conflict before and
        // it's still the same commit then skip it.
        if conflicts.branches.get(&branch.branch).map(|s| s.as_str()) == Some(branch_commit) {
            eprintln!("{}", "• Skipping rebase because it had conflicts last time we tried; rebase manually".yellow());
            continue;
        }

        conflicts.branches.remove(&branch.branch);
        conflicts.write_to_file(&conflicts_path)?;

        // Get the list of commits we will try to rebase onto (starting with `onto_branch`).
        let target_commit_list = get_target_commit_list(&repo_path, &branch.branch, onto_branch)?;

        // Check out the branch, unless it is already checked out.
        if branch.worktree.is_none() {
            checkout_branch(&branch.branch, &worktree_path)?;
        }

        // Path where we'll do the rebase.
        let rebase_worktree_path = if let Some(worktree) = &branch.worktree {
            &worktree.path
        } else {
            &worktree_path
        };

        let mut stopped_by_conflicts = false;

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

        // Detach HEAD in the scratch worktree and ensure it is on a commit that
        // we want to keep. This ensures that
        //
        //   a) If we did the rebase in the scratch worktree then the branch can
        //      be checked out elsewhere, and
        //   b) If we did the rebase in the user's worktree then the scratch
        //      worktree doesn't keep a reference to the commit around weirdly
        //      (it shows up in `git log` with no refs).
        //
        // This doesn't run afoul of git's rule about checking out the branch
        // in more than one worktree because of `--detach`.
        // to the old pre-rebase commit.
        git(&["checkout", "--detach", &branch.branch], &worktree_path)?;

        if stopped_by_conflicts {
            eprintln!("{}", "    - Rebase stunted by conflicts. Rebase manually.".yellow());

            // Get the commit again because it will have changed (probably).
            let new_branch_commit = git(&["rev-parse", &branch.branch], repo_path)?.stdout;
            let new_branch_commit = std::str::from_utf8(new_branch_commit.trim_ascii_whitespace())?;

            conflicts.branches.insert(branch.branch.clone(), new_branch_commit.to_owned());
            conflicts.write_to_file(&conflicts_path)?;
        }
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
    let worktree_path = worktree_path.to_str().ok_or_else(|| anyhow!("worktree path is not unicode"))?;
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

    let output = git(&["for-each-ref", "--format=%(refname:short)%00%(upstream:short)%00%(worktreepath)", "refs/heads"], repo_path)?.stdout;
    let branches = output.split(|c| *c == b'\n').filter(
        |line| !line.is_empty()
    ).map(
        |line| {
            let parts: Vec<&[u8]> = line.split(|c| *c == 0).collect();
            if parts.len() != 3 {
                bail!("for-each-ref parse error, got {} parts, expected 3", parts.len());
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
                Some(WorktreeInfo {
                    path,
                    clean,
                })
            };

            Ok(BranchInfo {
                branch,
                upstream,
                worktree,
            })
        }
    ).collect::<Result<_, _>>()?;
    Ok(branches)
}

fn get_merge_base(repo_path: &Path, a: &str, b: &str) -> Result<String> {
    let output = git(&["merge-base", a, b], repo_path)?.stdout;
    let output = std::str::from_utf8(output.trim_ascii_whitespace())?;
    Ok(output.to_owned())
}

fn checkout_branch(branch: &str, repo_path: &Path) -> Result<()> {
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

    git(&["diff-index", "--quiet", "HEAD"], worktree_path).is_ok() &&
    git(&["diff-index", "--quiet", "--cached", "HEAD"], worktree_path).is_ok()
}

enum RebaseResult {
    Success,
    Conflict,
}

fn attempt_rebase(repo_path: &Path, worktree_path: &Path, onto: &str) -> Result<RebaseResult> {
    let rebase_ok = git(&["rebase", onto], worktree_path);
    if rebase_ok.is_ok() {
        return Ok(RebaseResult::Success)
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

fn get_target_commit_list(repo_path: &Path, branch: &str, onto: &str) -> Result<Vec<String>> {
    let merge_base = get_merge_base(repo_path, branch, onto)?;

    let output = git(&["log", "--format=%H", &format!("{}..{}", merge_base, onto)], repo_path)?.stdout;
    let output = String::from_utf8(output)?;
    Ok(output.lines().map(ToOwned::to_owned).collect())
}
