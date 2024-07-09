use crate::{commit_graph, utils::*};
use autorebase::autorebase;

// Test building a repo using `build_repo`.
#[test]
fn test_build_repo() {
    git_fixed_dates();

    let root = commit("Hello")
        .write("a.txt", "hello")
        .child(commit("World").write("a.txt", "world").branch("master"));

    let repo = build_repo(&root, Some("master"));

    let repo_dir = repo.path();

    print_git_log_graph(repo_dir);

    let graph = get_repo_graph(repo_dir).expect("error getting repo graph");

    let expected_graph = commit_graph!(
        "baf6cf8e026e065d369b3dd103c4cc73ffba52dd": CommitGraphNode {
            parents: [
                "fdc071d3ae2b15728ab5a20d32b2c781999238ba",
            ],
            refs: {
                "master",
            },
        },
        "fdc071d3ae2b15728ab5a20d32b2c781999238ba": CommitGraphNode {
            parents: [],
            refs: {
                "",
            },
        },
    );

    assert_eq!(graph, expected_graph);
}

// Very basic autorebase test.
#[test]
fn basic_autorebase_slow() {
    basic_autorebase(true);
}

#[test]
fn basic_autorebase_fast() {
    basic_autorebase(false);
}

fn basic_autorebase(slow_conflict_detection: bool) {
    git_fixed_dates();

    let root = commit("First")
        .write("a.txt", "hello")
        .child(commit("Second").write("a.txt", "world").branch("master"))
        .child(commit("WIP").write("b.txt", "foo").branch("wip"));

    let repo = build_repo(&root, Some("master"));

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
