use autorebase::autorebase;
use crate::utils::*;

// Test randomly generated repos.
#[test]
fn random_test_slow() {
    random_test(true);
}

#[test]
fn random_test_fast() {
    random_test(false);
}

fn random_test(slow_conflict_detection: bool) {
    git_fixed_dates();

    let root = random_repo(false);

    let repo = build_repo(&root, Some("master"));

    let repo_dir = repo.path();

    print_git_log_graph(&repo_dir);

    autorebase(repo_dir, "master", slow_conflict_detection).expect("error autorebasing");

    print_git_log_graph(&repo_dir);

    // This doesn't really test anything yet; just makes sure the code doesn't panic.
}

#[test]
fn random_test_many_slow() {
    random_test_many(true);
}

#[test]
fn random_test_many_fast() {
    random_test_many(false);
}

fn random_test_many(slow_conflict_detection: bool) {

    // This takes about 0.5 seconds per iteration.
    for _ in 0..10 {
        random_test(slow_conflict_detection);
    }
}
