use crate::{commit_graph, utils::*};
use autorebase::autorebase;

// Single branch that cannot be rebased all the way to `master` commit due to conflicts.
#[test]
fn conflict_slow() {
    conflict(true);
}

#[test]
fn conflict_fast() {
    conflict(false);
}

fn conflict(slow_conflict_detection: bool) {
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

    let repo = build_repo(&root, Some("master"));

    let repo_dir = repo.path();

    print_git_log_graph(&repo_dir);

    autorebase(
        repo_dir,
        Some("master"),
        slow_conflict_detection,
        false,
        None,
    )
    .expect("error autorebasing");

    print_git_log_graph(&repo_dir);

    let graph = get_repo_graph(&repo_dir).expect("error getting repo graph");

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
