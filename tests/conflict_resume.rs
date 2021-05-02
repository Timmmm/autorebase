mod utils;
use utils::*;
use autorebase::autorebase;
use git_commands::git;
use std::fs;

// Single branch that cannot be rebased all the way to `master` commit due to conflicts,
// However we then change master so there's no conflict, but when we run `autorebase`
// again it should do nothing because it has remembered that the branch was blocked
// by conflicts. Finally we modify the branch which should cause it to attempt
// a rebase again when we run `autorebase` for the third time.
#[test]
fn conflict_resume() {
    git_fixed_dates();

    let root =
        commit("First")
        .write("a.txt", "hello")
        .child(
            commit("Second")
            .write("a.txt", "world")
            .child(
                commit("Third")
                .write("b.txt", "and")
                .child(
                    commit("Fourth")
                    .write("b.txt", "others")
                    .branch("master")
                )
            )
        )
        .child(
            commit("WIP")
            .write("b.txt", "goodbye")
            .branch("wip")
        );

    // It should rebase `wip` to the `Second` commit and then mark it as blocked.

    let repo = build_repo(&root, Some("master"));

    let repo_dir = repo.path();

    print_git_log_graph(&repo_dir);

    autorebase(repo_dir, "master").expect("error autorebasing");

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

    // Now modify master so there's no conflict.
    git(&["checkout", "master"], repo_dir).expect("error checking out master");
    fs::remove_file(repo_dir.join("b.txt")).expect("error removing file");
    git(&["add", "."], repo_dir).expect("error adding .");
    git(&["commit", "-m", "Remove conflict"], repo_dir).expect("error committing");

    // Ok if we run `autorebase` again we should expect it not to change anything.

    print_git_log_graph(&repo_dir);

    autorebase(repo_dir, "master").expect("error autorebasing");

    print_git_log_graph(&repo_dir);

    let graph = get_repo_graph(&repo_dir).expect("error getting repo graph");

    let expected_graph = commit_graph!(
        "211ae909a7bf0a2052009b8c21bebc6947591277": CommitGraphNode {
            parents: [
                "698624a3383d0143790b469946feb93a2dc9d7d6",
            ],
            refs: {
                "master",
            },
        },
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
                "",
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

    // Now make an unrelated commit on the `wip` branch.
    git(&["checkout", "wip"], repo_dir).expect("error checking out wip");
    fs::write(repo_dir.join("c.txt"), "unrelated").expect("error writing file");
    git(&["add", "."], repo_dir).expect("error adding .");
    git(&["commit", "-m", "Unrelated change"], repo_dir).expect("error committing");

    // Check out master again so `wip` can be autorebased.
    git(&["checkout", "master"], repo_dir).expect("error checking out master");


    // Ok if we run `autorebase` is should succesfully rebase to master.

    print_git_log_graph(&repo_dir);

    autorebase(repo_dir, "master").expect("error autorebasing");

    print_git_log_graph(&repo_dir);

    let graph = get_repo_graph(&repo_dir).expect("error getting repo graph");

    let expected_graph = commit_graph!(
        "20b78857d9e5095469d9f91a5de77d0c36813b46": CommitGraphNode {
            parents: [
                "f1fb4d2e3826e35c6c99e9133ba5dc99d80cdeb0",
            ],
            refs: {
                "wip",
            },
        },
        "211ae909a7bf0a2052009b8c21bebc6947591277": CommitGraphNode {
            parents: [
                "698624a3383d0143790b469946feb93a2dc9d7d6",
            ],
            refs: {
                "master",
            },
        },
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
                "",
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
        "f1fb4d2e3826e35c6c99e9133ba5dc99d80cdeb0": CommitGraphNode {
            parents: [
                "211ae909a7bf0a2052009b8c21bebc6947591277",
            ],
            refs: {
                "",
            },
        },
    );
    assert_eq!(graph, expected_graph);
}
