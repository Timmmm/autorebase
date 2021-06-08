use tempfile::TempDir;

/// Generate a completely random repo with random commits, branches, etc.
/// If `allow_merges` is true then the repo may contain merge commits.
fn random_repo(
    allow_merges: bool
) -> TempDir {
    if allow_merges {
        todo!();
    }

    // let root =
    // commit("First")
    // .write("a.txt", "hello")
    // .child(
    //     commit("Second")
    //     .write("a.txt", "world")
    //     .branch("master")
    // )
    // .child(
    //     commit("WIP")
    //     .write("b.txt", "foo")
    //     .branch("wip")
    // );

    todo!();
}
