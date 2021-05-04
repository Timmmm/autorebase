use tempfile::{TempDir, tempdir};
use std::fs::{write, remove_file};
use std::collections::{HashMap, BTreeMap, BTreeSet};
use git_commands::*;
use std::path::Path;
use anyhow::{anyhow, Result};

/// Set environment variables so Git uses fixed dates. This ensures the
/// hashes are deterministic which is useful for tests.
pub fn git_fixed_dates() {
    std::env::set_var("GIT_AUTHOR_DATE", "@0 +0000");
    std::env::set_var("GIT_COMMITTER_DATE", "@0 +0000");
}

/// Create a temporary directory and initialise it as a Git repo.
pub fn create_temporary_git_repo() -> TempDir {
    let repo_dir = tempdir().expect("Couldn't create temporary directory");
    git(&["init", "--initial-branch=master"], &repo_dir.path()).expect("error initialising git repo");
    // You have to set these otherwise Git can't do commits.
    git(&["config", "user.email", "me@example.com"], &repo_dir.path()).expect("error setting config");
    git(&["config", "user.name", "Me"], &repo_dir.path()).expect("error setting config");
    // Hide detached head warnings.
    git(&["config", "advice.detachedHead", "false"], &repo_dir.path()).expect("error setting config");
    repo_dir
}

// Run `git log` to show the commit graph.
pub fn print_git_log_graph(repo_dir: &Path) {
    let out = git(&["--no-pager", "log", "--oneline", "--decorate", "--graph", "--all", "--color=always"], repo_dir).expect("git log failed").stdout;
    println!("\n{}\n", String::from_utf8_lossy(&out));
}

/// A commit description, used to build Git repos.
#[derive(Default)]
pub struct CommitDescription {
    /// The commit message.
    message: String,
    /// Map from filename to the new contents or None to delete it.
    changes: HashMap<String, Option<String>>,
    /// Branch names on this commit.
    branches: Vec<String>,
    /// Child commits.
    children: Vec<CommitDescription>,
    /// ID, only used for merge_parents.
    id: Option<i32>,
    /// Additional parents for merge commits.
    merge_parents: Vec<i32>,
}

/// Builder methods to set fields.
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

/// Helper function to create a new `CommitDescription`.
pub fn commit(message: &str) -> CommitDescription {
    CommitDescription {
        message: message.to_owned(),
        ..Default::default()
    }
}

/// Create a new git repo in a temporary directory with contents described by the
/// `root` commit and its children. When done if `checkout_when_done` is not `None`
/// it will be checked out. Otherwise the repo will have a detached HEAD on one
/// of the leaf commits.
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

        git(&["add", "."], repo_path).expect("error adding changes");
        let tree_object = git(&["write-tree"], repo_path).expect("error writing tree").stdout;
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
        let this_commit = git(&args, repo_path).expect("error committing").stdout;
        let this_commit = String::from_utf8_lossy(&this_commit);
        let this_commit = this_commit.trim();

        // Check it out in case we want to set branches.
        git(&["checkout", this_commit], repo_path).expect("error checking out commit");

        // Set branches.
        for branch in c.branches.iter() {
            git(&["branch", branch], repo_path).expect("error setting branch");
        }

        if let Some(id) = c.id {
            hash_by_id.insert(id, this_commit.to_string());
        }

        // Process the children.
        for child in c.children.iter() {
            // Checkout this commit.
            git(&["checkout", &this_commit], repo_path).expect("error checking out commit");
            process_commit(child, repo_path, Some(&this_commit), hash_by_id);
        }
    }

    let mut hash_by_id = HashMap::new();

    process_commit(root, repo.path(), None, &mut hash_by_id);

    if let Some(checkout_when_done) = checkout_when_done {
        git(&["checkout", checkout_when_done], repo.path()).expect("couldn't check out final thing");
    }

    repo
}

/// A node in the commit graph (i.e. a commit!) used for testing whether the
/// graph is correct.
#[derive(Default, Debug, PartialEq, Eq)]
pub struct CommitGraphNode {
    // pub subject: String,
    pub parents: Vec<String>,
    pub refs: BTreeSet<String>,
}

pub fn get_repo_graph(repo_dir: &Path) -> Result<BTreeMap<String, CommitGraphNode>> {
    // git log --all --format='%H%x00%P%x00%D%x00%s'
    //
    // gives <commit_hash>\0<parent_hashes>\0<refs>\0<subject>
    //
    // Probably have to do separate commands to get the different bits.

    // Then build the graph structure.

    let mut commits: BTreeMap<String, CommitGraphNode> = BTreeMap::new();

    let commit_parents = git(&["log", "--all", "--format=%H %P"], repo_dir)?.stdout;
    let commit_parents = String::from_utf8_lossy(&commit_parents);
    for line in commit_parents.lines() {
        let mut parts = line.split_ascii_whitespace();
        let commit = parts.next().ok_or(anyhow!("invalid git log output"))?;
        commits.entry(commit.to_string()).or_default().parents = parts.map(|p| p.to_owned()).collect();
    }

    let commit_refs = git(&["log", "--all", "--format=%H,%D"], repo_dir)?.stdout;
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

    // let commit_subject = git(&["log", "--all", "--format=%H%x00%s"], repo_dir)?.stdout;
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

/// A helper macro to create a `BTreeMap<String, CommitGraphNode>` from the
/// corresponding `dbg!()` output.
#[macro_export]
macro_rules! commit_graph {
    (
        $(
            $hash:literal : CommitGraphNode {
                $($field_name:ident : $field_value:tt),*
                $(,)?
            }
        ),*
        $(,)?
    ) => {{
        #[allow(unused_mut)]
        let mut graph: ::std::collections::BTreeMap<String, CommitGraphNode> = ::std::collections::BTreeMap::new();
        $(
            graph.insert($hash.to_string(), {
                #[allow(unused_mut)]
                let mut info: CommitGraphNode = Default::default();
                $(
                    commit_graph!(@set_field info, $field_name, $field_value);
                )*
                info
            });
        )*
        graph
    }};

    (@set_field $object:ident, parents, [ $($hash:expr),* $(,)? ]) => {
        $object.parents = vec![$($hash.to_string(), )*];
    };

    (@set_field $object:ident, refs, { $($hash:literal),* $(,)? }) => {
        $object.refs = {
            #[allow(unused_mut)]
            let mut r = ::std::collections::BTreeSet::new();
            $(
                r.insert($hash.to_string());
            )*
            r
        };
    };
}
