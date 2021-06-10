use super::CommitDescription;
use rand::Rng;
use std::collections::HashSet;

/// Generate a completely random repo with random commits, branches, etc.
/// If `allow_merges` is true then the repo may contain merge commits.
pub fn random_repo(allow_merges: bool) -> CommitDescription {
    if allow_merges {
        todo!();
    }

    // This uses recursion so we need to set a maximum depth to avoid stack overflows.

    fn randomise_commit(
        commit: &mut CommitDescription,
        branches: &mut HashSet<String>,
        depth: u32,
    ) {
        let mut rng = rand::thread_rng();

        // Randomly set the name, branch, contents, etc.
        commit.message = format!("Commit {}", rng.gen_range(0..1000));
        // The filename and contents are drawn from a small distribution to
        // give a reasonable chance of writing the same file to make conflicts,
        // or serendipitously eliminating conflicts.
        commit.changes.insert(
            format!("{}.txt", rng.gen_range(0..8)),
            Some(format!("{}", rng.gen_range(0..8))),
        );
        if rng.gen_bool(0.1) {
            let branch_name = format!("branch_{}", rng.gen_range(0..1000000));
            if !branches.contains(&branch_name) {
                branches.insert(branch_name.clone());
                commit.branches.push(branch_name);
            }
        }

        // Possibly add one or more children. Most commonly we add exactly one
        // child. Less commonly we add 0, or 2 or more. The expected number
        // must be less than 1!
        let num_children = rng.gen_range(0..100);
        let num_children = if depth > 100 || num_children < 30 {
            0
        } else if num_children < 90 {
            1
        } else if num_children < 95 {
            2
        } else {
            3
        };

        // So that we guarantee something is `master`, the first tip will be master.
        if !branches.contains("master") {
            commit.branches.push("master".to_owned());
            branches.insert("master".to_owned());
        }

        for _ in 0..num_children {
            // Add a child commit
            commit.children.push(Default::default());
            randomise_commit(commit.children.last_mut().unwrap(), branches, depth + 1);
        }
    }

    let mut root = Default::default();
    let mut branches = Default::default();

    randomise_commit(&mut root, &mut branches, 0);

    root
}
