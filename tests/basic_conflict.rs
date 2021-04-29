mod utils;
use utils::*;
use autorebase::autorebase;

// Single branch that cannot be rebased all the way to `master` commit due to conflicts.
#[test]
fn conflict() {
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

    dbg!(&graph);

    let expected_graph = commit_graph!(

    );
    assert_eq!(graph, expected_graph);
}
