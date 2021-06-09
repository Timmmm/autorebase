mod utils;
use utils::*;

// Not actually a test; just generate a repo for demonstration purposes.
//
// Run `cargo test create_test_repo -- --nocapture --include-ignored`
#[ignore]
#[test]
fn create_test_repo() {
    git_fixed_dates();

    let root = commit("Hello")
        .write("a.txt", "hello")
        .child(commit("World").write("a.txt", "world").branch("master"));

    let repo = build_repo(&root, Some("master"));

    let repo_dir = repo.into_path();

    print_git_log_graph(&repo_dir);

    println!("Repo at: {:?}", repo_dir);
}
