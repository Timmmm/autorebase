mod utils;
use utils::*;
use autorebase::autorebase;

// Basic test but there are multiple chained refs on the branch.
#[test]
fn multiple_branches() {
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
            commit("WIP 1")
            .write("b.txt", "foo1")
            .branch("wip1")
            .child(
                commit("WIP 2")
                .write("b.txt", "foo2")
                .branch("wip2")
            )
        );

    let repo = build_repo(&root, Some("master"));

    let repo_dir = repo.path();

    print_log(&repo_dir);

    autorebase(repo_dir, "master").expect("error autorebasing");

    print_log(&repo_dir);

    let graph = get_repo_graph(&repo_dir).expect("error getting repo graph");

    let expected_graph = commit_info_graph!(
        "540f822d14ae077991e2a722996825e4e7f9d667": CommitInfo {
            parents: [
                "a6de41485a5af44adc18b599a63840c367043e39",
            ],
            refs: {
                "wip1",
            },
        },
        "a6de41485a5af44adc18b599a63840c367043e39": CommitInfo {
            parents: [
                "d3591307bd5590f14ae24d03ab41121ab94e2a90",
            ],
            refs: {
                "master",
            },
        },
        "b5656b97e2a114800e6bd909e3cc5b3db3602e35": CommitInfo {
            parents: [
                "540f822d14ae077991e2a722996825e4e7f9d667",
            ],
            refs: {
                "wip2",
            },
        },
        "d3591307bd5590f14ae24d03ab41121ab94e2a90": CommitInfo {
            parents: [],
            refs: {
                "",
            },
        },
    );
    assert_eq!(graph, expected_graph);
}
