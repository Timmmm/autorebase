// Tool to automatically rebase branches.

use argh::FromArgs;
use std::path::{Path, PathBuf};
use anyhow::{anyhow, Result};
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

fn is_rebasing(repo_dir: &Path) -> Result<bool> {
    // Check `.git/rebase-merge` exists. See https://stackoverflow.com/questions/3921409/how-to-know-if-there-is-a-git-rebase-in-progress/67245016#67245016
    todo!()
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
    use std::fs::{write, remove_file};
    use super::*;
    use std::process::Command;
    use std::collections::HashMap;

    fn create_temporary_git_repo() -> TempDir {
        let repo_dir = tempdir().expect("Couldn't create temporary directory");
        run_git_cmd(&["init", "--initial-branch=master"], &repo_dir.path()).expect("error initialising git repo");
        // You have to set these otherwise Git can't do commits.
        run_git_cmd(&["config", "user.email", "me@example.com"], &repo_dir.path()).expect("error setting config");
        run_git_cmd(&["config", "user.name", "Me"], &repo_dir.path()).expect("error setting config");
        // Hide detached head warnings.
        run_git_cmd(&["config", "advice.detachedHead", "false"], &repo_dir.path()).expect("error setting config");
        repo_dir
    }

    fn do_commit(message: &str, working_dir: &Path) {
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

    #[derive(Default)]
    struct CommitDescription {
        message: String,
        // Map from filename to the new contents or None to delete it.
        changes: HashMap<String, Option<String>>,
        // Branch names on this commit.
        branches: Vec<String>,
        // Child commits.
        children: Vec<CommitDescription>,
        // ID, only used for merge_parents.
        id: Option<i32>,
        // Additional parents for merge commits.
        merge_parents: Vec<i32>,
    }

    #[allow(dead_code)]
    impl CommitDescription {
        fn message(mut self, m: &str) -> Self {
            self.message = m.to_string();
            self
        }
        fn write(mut self, filename: &str, contents: &str) -> Self {
            self.changes.insert(filename.to_owned(), Some(contents.to_owned()));
            self
        }
        fn delete(mut self, filename: &str) -> Self {
            self.changes.insert(filename.to_owned(), None);
            self
        }
        fn branch(mut self, branch_name: &str) -> Self {
            self.branches.push(branch_name.to_owned());
            self
        }
        fn child(mut self, commit: CommitDescription) -> Self {
            self.children.push(commit);
            self
        }
        fn id(mut self, id: i32) -> Self {
            self.id = Some(id);
            self
        }
        fn merge_parent(mut self, parent_id: i32) -> Self {
            self.merge_parents.push(parent_id);
            self
        }
    }

    fn commit(message: &str) -> CommitDescription {
        CommitDescription {
            message: message.to_owned(),
            ..Default::default()
        }
    }

    fn build_repo(root: &CommitDescription) -> TempDir {
        let repo = create_temporary_git_repo();

        // First assign IDs to the commits that don't have any (and verify
        // no duplicates).

        // Then store a map from id to git hash.

        // Then each time we can just check out the commit detached,
        // add all the children, then recurse to the children.

        fn process_commit(c: &CommitDescription, repo_path: &Path, parent: Option<&str>, hash_by_id: &mut HashMap<i32, String>) {
            // Make changes to filesystem.
            for (path, change) in &c.changes {
                let path = repo_path.join(path);
                match change {
                    Some(contents) => {
                        eprintln!("  Write {:?}", path);
                        write(&path, contents).expect("error writing file");
                    }
                    None => {
                        eprintln!("  Delete {:?}", path);
                        remove_file(&path).expect("error removing file");
                    }
                }
            }

            run_git_cmd(&["add", "."], repo_path).expect("error adding changes");
            let tree_object = run_git_cmd_output(&["write-tree"], repo_path).expect("error writing tree");
            let tree_object = String::from_utf8_lossy(&tree_object);
            let tree_object = tree_object.trim();

            // Build `git commit-tree` args.
            let mut args = vec!["commit-tree", "-m", &c.message];
            if let Some(parent) = parent {
                args.push("-p");
                args.push(parent);
            }
            for parent_id in c.merge_parents.iter() {
                let parent = hash_by_id.get(parent_id).expect("ID not found");
                args.push("-p");
                args.push(parent);
            }

            args.push(&tree_object);

            // Commit.
            let this_commit = run_git_cmd_output(&args, repo_path).expect("error committing");
            let this_commit = String::from_utf8_lossy(&this_commit);
            let this_commit = this_commit.trim();

            // Check it out in case we want to set branches.
            run_git_cmd(&["checkout", this_commit], repo_path).expect("error checking out commit");

            // Set branches.
            for branch in c.branches.iter() {
                run_git_cmd(&["branch", branch], repo_path).expect("error setting branch");
            }

            if let Some(id) = c.id {
                hash_by_id.insert(id, this_commit.to_string());
            }

            // Process the children.
            for child in c.children.iter() {
                // Checkout this commit.
                run_git_cmd(&["checkout", &this_commit], repo_path).expect("error checking out commit");
                process_commit(child, repo_path, Some(&this_commit), hash_by_id);
            }
        }

        let mut hash_by_id = HashMap::new();

        process_commit(root, repo.path(), None, &mut hash_by_id);

        repo
    }

    #[test]
    fn test_build_repo() {
        let root =
            commit("Hello")
            .write("a.txt", "hello")
            .child(
                commit("World")
                .write("a.txt", "world")
                .branch("master")
            );

        let repo = build_repo(&root);

        let repo_dir = repo.into_path(); // Keep the temporary directory.

        print_log(&repo_dir);
    }

    #[test]
    fn basic_test() {
        let repo = create_temporary_git_repo();
        let repo_dir = repo.into_path();

        eprintln!("Running test in {:?}", &repo_dir);

        write(&repo_dir.join("data1.txt"), "1").expect("unable to write file");
        run_git_cmd(&["add", "data1.txt"], &repo_dir).expect("git command failed");
        do_commit("Commit 1\n\nMore info", &repo_dir);

        // Checkout a new branch
        run_git_cmd(&["checkout", "-b", "two"], &repo_dir).expect("git command failed");

        write(&repo_dir.join("data2.txt"), "2").expect("unable to write file");
        run_git_cmd(&["add", "data2.txt"], &repo_dir).expect("git command failed");
        do_commit("Commit 2\n\nMore info", &repo_dir);

        // Go back to master.
        run_git_cmd(&["checkout", "master"], &repo_dir).expect("git command failed");

        write(&repo_dir.join("data3.txt"), "3").expect("unable to write file");
        run_git_cmd(&["add", "data3.txt"], &repo_dir).expect("git command failed");
        do_commit("Commit 3\n\nMore info", &repo_dir);

        // Log before.
        print_log(&repo_dir);

        // Auto-rebase!
        autorebase(&repo_dir, "master").expect("autorebase failed");

        // Log after.
        print_log(&repo_dir);
    }

    fn get_repo_graph(repo_dir: &Path) -> Result<()> {
        // git log --all --format='%H%x00%P%x00%D%x00%s'
        //
        // gives <commit_hash>\0<parent_hashes>\0<refs>\0<subject>
        //
        // Probably have to do separate commands to get the different bits.

        // Then build the graph structure.

        #[derive(Default)]
        struct CommitInfo {
            parents: Vec<String>,
            refs: Vec<String>,
            subject: String,
        }

        let mut commits: HashMap<String, CommitInfo> = HashMap::new();

        let commit_parents = run_git_cmd_output(&["log", "--all", "--format=%H %P"], repo_dir)?;
        let commit_parents = String::from_utf8_lossy(&commit_parents);
        for line in commit_parents.lines() {
            let mut parts = line.split_ascii_whitespace();
            let commit = parts.next().ok_or(anyhow!("invalid git log output"))?;
            commits.entry(commit.to_string()).or_default().parents = parts.map(|p| p.to_owned()).collect();
        }

        let commit_refs = run_git_cmd_output(&["log", "--all", "--format=%H,%D"], repo_dir)?;
        let commit_refs = String::from_utf8_lossy(&commit_refs);
        for line in commit_refs.lines() {
            let mut parts = line.split(',');
            let commit = parts.next().ok_or(anyhow!("invalid git log output"))?;
            commits.entry(commit.to_string()).or_default().refs = parts.map(
                |p| p.trim().trim_start_matches("HEAD -> ").to_owned()
            ).collect();
        }


        let commit_subject = run_git_cmd_output(&["log", "--all", "--format=%H%x00%s"], repo_dir)?;
        let commit_subject = String::from_utf8_lossy(&commit_subject);
        for line in commit_subject.lines() {
            let mut parts = line.split('\x00');
            let commit = parts.next().ok_or(anyhow!("invalid git log output"))?;
            commits.entry(commit.to_string()).or_default().subject = parts.next().ok_or(anyhow!("invalid git log subject output"))?.to_owned();
        }

        todo!()
    }



}
