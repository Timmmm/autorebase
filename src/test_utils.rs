use tempfile::{TempDir, tempdir};
use std::fs::{write, remove_file};
use std::collections::{HashMap, BTreeMap, BTreeSet};
use crate::git_commands::*;
use std::path::Path;
use anyhow::{anyhow, Result};

pub fn create_temporary_git_repo() -> TempDir {
    let repo_dir = tempdir().expect("Couldn't create temporary directory");
    run_git_cmd(&["init", "--initial-branch=master"], &repo_dir.path()).expect("error initialising git repo");
    // You have to set these otherwise Git can't do commits.
    run_git_cmd(&["config", "user.email", "me@example.com"], &repo_dir.path()).expect("error setting config");
    run_git_cmd(&["config", "user.name", "Me"], &repo_dir.path()).expect("error setting config");
    // Hide detached head warnings.
    run_git_cmd(&["config", "advice.detachedHead", "false"], &repo_dir.path()).expect("error setting config");
    repo_dir
}

pub fn print_log(repo_dir: &Path) {
    run_git_cmd(&["--no-pager", "log", "--oneline", "--decorate", "--graph", "--all"], repo_dir).expect("git log failed");
}

#[derive(Default)]
pub struct CommitDescription {
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
    pub fn message(mut self, m: &str) -> Self {
        self.message = m.to_string();
        self
    }
    pub fn write(mut self, filename: &str, contents: &str) -> Self {
        self.changes.insert(filename.to_owned(), Some(contents.to_owned()));
        self
    }
    pub fn delete(mut self, filename: &str) -> Self {
        self.changes.insert(filename.to_owned(), None);
        self
    }
    pub fn branch(mut self, branch_name: &str) -> Self {
        self.branches.push(branch_name.to_owned());
        self
    }
    pub fn child(mut self, commit: CommitDescription) -> Self {
        self.children.push(commit);
        self
    }
    pub fn id(mut self, id: i32) -> Self {
        self.id = Some(id);
        self
    }
    pub fn merge_parent(mut self, parent_id: i32) -> Self {
        self.merge_parents.push(parent_id);
        self
    }
}

pub fn commit(message: &str) -> CommitDescription {
    CommitDescription {
        message: message.to_owned(),
        ..Default::default()
    }
}

pub fn build_repo(root: &CommitDescription, checkout_when_done: Option<&str>) -> TempDir {
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

        // Commit. Set the date to a fixed value so we get the same output
        // each time.
        let this_commit = run_git_cmd_output_1970(&args, repo_path).expect("error committing");
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

    if let Some(checkout_when_done) = checkout_when_done {
        run_git_cmd(&["checkout", checkout_when_done], repo.path()).expect("couldn't check out final thing");
    }

    repo
}


#[derive(Default, Debug, PartialEq, Eq)]
pub struct CommitInfo {
    // pub subject: String,
    pub parents: Vec<String>,
    pub refs: BTreeSet<String>,
}

pub fn get_repo_graph(repo_dir: &Path) -> Result<BTreeMap<String, CommitInfo>> {
    // git log --all --format='%H%x00%P%x00%D%x00%s'
    //
    // gives <commit_hash>\0<parent_hashes>\0<refs>\0<subject>
    //
    // Probably have to do separate commands to get the different bits.

    // Then build the graph structure.

    let mut commits: BTreeMap<String, CommitInfo> = BTreeMap::new();

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
        commits.entry(commit.to_string()).or_default().refs = parts.filter_map(
            |p| {
                let p = p.trim().trim_start_matches("HEAD -> ");
                if p == "HEAD" {
                    None
                } else {
                    Some(p.to_owned())
                }
            }
        ).collect();
    }

    // let commit_subject = run_git_cmd_output(&["log", "--all", "--format=%H%x00%s"], repo_dir)?;
    // let commit_subject = String::from_utf8_lossy(&commit_subject);
    // for line in commit_subject.lines() {
    //     let mut parts = line.split('\x00');
    //     let commit = parts.next().ok_or(anyhow!("invalid git log output"))?;
    //     commits.entry(commit.to_string()).or_default().subject = parts.next().ok_or(anyhow!("invalid git log subject output"))?.to_owned();
    // }

    // This is sufficient to check the structure - we just check for equality.
    // Rather than doing complicated hash-agnostic graph isomorphism stuff
    // we just arrange things so the hashes are the same every run.

    Ok(commits)
}
