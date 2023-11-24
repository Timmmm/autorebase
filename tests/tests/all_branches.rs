use std::collections::BTreeMap;

use autorebase::autorebase;

use crate::{commit_graph, utils::*};

fn with_include(include_all_branches: bool) -> BTreeMap<String, CommitGraphNode> {
    git_fixed_dates();

    let root = commit("First")
        .write("a.txt", "hello")
        .child(commit("Second").write("a.txt", "world").branch("master"))
        .child(
            commit("Third")
                .write("b.txt", "foo")
                .branch_with_upstream("other_main", "master"),
        )
        .child(commit("wip").write("c.txt", "foo").branch("wip"));

    let repo = build_repo(&root, Some("master"));

    let repo_dir = repo.path();

    print_git_log_graph(&repo_dir);

    autorebase(repo_dir, Some("master"), false, include_all_branches, None).expect("error autorebasing");

    print_git_log_graph(&repo_dir);

    get_repo_graph(&repo_dir).expect("error getting repo graph")
}

#[test]
fn skips_branches() {
    let graph = with_include(false);

    let expected_graph = commit_graph!(
        "6781625b397d4f2eeb6da4b1fea570052683629f": CommitGraphNode {
            parents: ["d3591307bd5590f14ae24d03ab41121ab94e2a90"],
            refs: {"other_main"},
        },
        "a6de41485a5af44adc18b599a63840c367043e39": CommitGraphNode {
            parents: ["d3591307bd5590f14ae24d03ab41121ab94e2a90"],
            refs: {"master"},
        },
        "d3591307bd5590f14ae24d03ab41121ab94e2a90": CommitGraphNode {
            parents: [],
            refs: {""},
        },
        "f7aad7ec74984d4cd89090e572de921d5f9d1fc4": CommitGraphNode {
            parents: ["a6de41485a5af44adc18b599a63840c367043e39"],
            refs: {"wip"}
        }
    );
    assert_eq!(graph, expected_graph);
}

#[test]
fn includes_branches() {
    let graph = with_include(true);

    let expected_graph = commit_graph!(
        "089f39ba0066fd2380da7dbe5201ec4b13f01b4a": CommitGraphNode {
            parents: ["a6de41485a5af44adc18b599a63840c367043e39"],
            refs: {"other_main"}
        },
        "a6de41485a5af44adc18b599a63840c367043e39": CommitGraphNode {
            parents: ["d3591307bd5590f14ae24d03ab41121ab94e2a90"],
            refs: {"master"}
        },
        "d3591307bd5590f14ae24d03ab41121ab94e2a90": CommitGraphNode {
            parents: [],
            refs: {""}
        },
        "f7aad7ec74984d4cd89090e572de921d5f9d1fc4": CommitGraphNode {
            parents: ["a6de41485a5af44adc18b599a63840c367043e39"],
            refs: {"wip"}
        }
    );
    assert_eq!(graph, expected_graph);
}
