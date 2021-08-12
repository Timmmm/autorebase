use crate::{commit_graph, utils::*};
use autorebase::autorebase;
use tempfile::tempdir;

// Test running autorebase from another worktree.
#[test]
fn from_another_worktree_slow() {
    from_another_worktree(true);
}

#[test]
fn from_another_worktree_fast() {
    from_another_worktree(false);
}

fn from_another_worktree(slow_conflict_detection: bool) {
    git_fixed_dates();

    let root = commit("First")
        .write("a.txt", "hello")
        .child(commit("Second").write("a.txt", "world").branch("master"))
        .child(commit("WIP").write("b.txt", "foo").branch("wip"));

    let repo = build_repo(&root, Some("master"));

    let repo_dir = repo.path();

    // Now create another worktree in a completely different directory.
    let another_worktree_dir = tempdir().expect("Couldn't create temporary directory");
    let another_worktree_repo_dir = another_worktree_dir.path().join("another");

    git_commands::git(
        &["worktree", "add", &another_worktree_repo_dir.to_str().expect("non-unicode test path")],
        repo_dir,
    ).expect("Couldn't create another worktree");

    print_git_log_graph(&repo_dir);

    // Now autorebase from the other worktree dir.
    autorebase(&another_worktree_repo_dir, "master", slow_conflict_detection, false).expect("error autorebasing");

    print_git_log_graph(&repo_dir);

    let graph = get_repo_graph(&repo_dir).expect("error getting repo graph");

    let expected_graph = commit_graph!(
        "a6de41485a5af44adc18b599a63840c367043e39": CommitGraphNode {
            parents: [
                "d3591307bd5590f14ae24d03ab41121ab94e2a90",
            ],
            refs: {
                "another", "master",
            },
        },
        "d3591307bd5590f14ae24d03ab41121ab94e2a90": CommitGraphNode {
            parents: [],
            refs: {
                "",
            },
        },
        "e42d214485dff70e93fdf6c66901b9ae4cc05b5a": CommitGraphNode {
            parents: [
                "a6de41485a5af44adc18b599a63840c367043e39",
            ],
            refs: {
                "wip",
            },
        },
    );
    assert_eq!(graph, expected_graph);
}
