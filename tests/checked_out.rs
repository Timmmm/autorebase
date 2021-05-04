
mod utils;
use utils::*;
use autorebase::autorebase;
use std::fs;

// Check we can rebase with the current checked out branch.
#[test]
fn checkedout_clean() {
    git_fixed_dates();

    let root =
        commit("First")
        .write("a.txt", "hello")
        .child(
            commit("Second")
            .write("a.txt", "world")
            .branch("master")
        )
        .child(
            commit("WIP")
            .write("b.txt", "foo")
            .branch("wip")
        );

    let repo = build_repo(&root, Some("wip"));

    let repo_dir = repo.path();

    print_git_log_graph(&repo_dir);

    autorebase(repo_dir, "master").expect("error autorebasing");

    print_git_log_graph(&repo_dir);

    let graph = get_repo_graph(&repo_dir).expect("error getting repo graph");

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
fn checkedout_dirty() {
    git_fixed_dates();

    let root =
        commit("First")
        .write("a.txt", "hello")
        .child(
            commit("Second")
            .write("a.txt", "world")
            .branch("master")
        )
        .child(
            commit("WIP")
            .write("b.txt", "foo")
            .branch("wip")
        );

    let repo = build_repo(&root, Some("wip"));

    let repo_dir = repo.path();

    // Make it dirty.
    fs::write(repo_dir.join("b.txt"), "baz").expect("error writing file");

    print_git_log_graph(&repo_dir);

    autorebase(repo_dir, "master").expect("error autorebasing");

    print_git_log_graph(&repo_dir);

    let graph = get_repo_graph(&repo_dir).expect("error getting repo graph");

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
