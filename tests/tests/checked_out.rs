use crate::{commit_graph, utils::*};
use autorebase::autorebase;
use std::fs;

// Check we can rebase with the current checked out branch.
#[test]
fn checkedout_clean_slow() {
    checkedout_clean(true);
}

#[test]
fn checkedout_clean_fast() {
    checkedout_clean(false);
}

fn checkedout_clean(slow_conflict_detection: bool) {
    git_fixed_dates();

    let root = commit("First")
        .write("a.txt", "hello")
        .child(commit("Second").write("a.txt", "world").branch("master"))
        .child(commit("WIP").write("b.txt", "foo").branch("wip"));

    let repo = build_repo(&root, Some("wip"));

    let repo_dir = repo.path();

    print_git_log_graph(repo_dir);

    autorebase(
        repo_dir,
        Some("master"),
        slow_conflict_detection,
        false,
        None,
    )
    .expect("error autorebasing");

    print_git_log_graph(repo_dir);

    let graph = get_repo_graph(repo_dir).expect("error getting repo graph");

    let expected_graph = commit_graph!(
        "a6de41485a5af44adc18b599a63840c367043e39": CommitGraphNode {
            parents: [
                "d3591307bd5590f14ae24d03ab41121ab94e2a90",
            ],
            refs: {
                "master",
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

// Nothing should happen in this case though because the tree is dirty.
#[test]
fn checkedout_dirty_slow() {
    checkedout_dirty(true);
}

#[test]
fn checkedout_dirty_fast() {
    checkedout_dirty(false);
}

fn checkedout_dirty(slow_conflict_detection: bool) {
    git_fixed_dates();

    let root = commit("First")
        .write("a.txt", "hello")
        .child(commit("Second").write("a.txt", "world").branch("master"))
        .child(commit("WIP").write("b.txt", "foo").branch("wip"));

    let repo = build_repo(&root, Some("wip"));

    let repo_dir = repo.path();

    // Make it dirty.
    fs::write(repo_dir.join("b.txt"), "baz").expect("error writing file");

    print_git_log_graph(repo_dir);

    autorebase(
        repo_dir,
        Some("master"),
        slow_conflict_detection,
        false,
        None,
    )
    .expect("error autorebasing");

    print_git_log_graph(repo_dir);

    let graph = get_repo_graph(repo_dir).expect("error getting repo graph");

    let expected_graph = commit_graph!(
        "a6de41485a5af44adc18b599a63840c367043e39": CommitGraphNode {
            parents: [
                "d3591307bd5590f14ae24d03ab41121ab94e2a90",
            ],
            refs: {
                "master",
            },
        },
        "d3591307bd5590f14ae24d03ab41121ab94e2a90": CommitGraphNode {
            parents: [],
            refs: {
                "",
            },
        },
        "dfff1861aaf18fc50834d9ded7178db9493a05ad": CommitGraphNode {
            parents: [
                "d3591307bd5590f14ae24d03ab41121ab94e2a90",
            ],
            refs: {
                "wip",
            },
        },
    );
    assert_eq!(graph, expected_graph);
}

// Single branch that cannot be rebased all the way to `master` commit due to conflicts.
// The branch is also checked out so that it is rebased in the user's worktree instead
// of our private one.
#[test]
fn checked_out_conflict_slow() {
    checked_out_conflict(true);
}

#[test]
fn checked_out_conflict_fast() {
    checked_out_conflict(false);
}

fn checked_out_conflict(slow_conflict_detection: bool) {
    git_fixed_dates();

    let root = commit("First")
        .write("a.txt", "hello")
        .child(
            commit("Second").write("a.txt", "world").child(
                commit("Third")
                    .write("b.txt", "and")
                    .child(commit("Fourth").write("b.txt", "others").branch("master")),
            ),
        )
        .child(commit("WIP").write("b.txt", "goodbye").branch("wip"));

    // It should rebase `wip` to the `Second` commit and then mark it as blocked.

    let repo = build_repo(&root, Some("wip"));

    let repo_dir = repo.path();

    print_git_log_graph(repo_dir);

    autorebase(
        repo_dir,
        Some("master"),
        slow_conflict_detection,
        false,
        None,
    )
    .expect("error autorebasing");

    print_git_log_graph(repo_dir);

    let graph = get_repo_graph(repo_dir).expect("error getting repo graph");

    let expected_graph = commit_graph!(
        "386e8eec713b111eca536adc310dfccf22323ad7": CommitGraphNode {
            parents: [
                "a6de41485a5af44adc18b599a63840c367043e39",
            ],
            refs: {
                "",
            },
        },
        "698624a3383d0143790b469946feb93a2dc9d7d6": CommitGraphNode {
            parents: [
                "386e8eec713b111eca536adc310dfccf22323ad7",
            ],
            refs: {
                "master",
            },
        },
        "808dd8d1d131ced226f3a9352251f2ed3d74b71c": CommitGraphNode {
            parents: [
                "a6de41485a5af44adc18b599a63840c367043e39",
            ],
            refs: {
                "wip",
            },
        },
        "a6de41485a5af44adc18b599a63840c367043e39": CommitGraphNode {
            parents: [
                "d3591307bd5590f14ae24d03ab41121ab94e2a90",
            ],
            refs: {
                "",
            },
        },
        "d3591307bd5590f14ae24d03ab41121ab94e2a90": CommitGraphNode {
            parents: [],
            refs: {
                "",
            },
        },
    );
    assert_eq!(graph, expected_graph);
}
