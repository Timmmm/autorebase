#[path = "../tests/utils/mod.rs"]
pub mod utils;
use utils::*;

// This program just creates an example repo so you can try out autorebase.
// It is created in a temporary directory that is printed at the end.
//
// To run: `cargo run --example create_demo_repo`
//
// To watch changes: `watchexec --no-default-ignore -w .git -- 'tput reset; git --no-pager log --oneline --all --graph --decorate'`
//
fn main() {
    git_fixed_dates();

    let root = commit("Initial commit")
        .write("a.txt", "hello")
        .child(
            commit("Write specification")
            .write("spec.txt", "Specifcation: Do nothing")
            .child(
                commit("Implement specification")
                .write("code.c", "int main() { return 1; }")
                .child(
                    commit("Fix bugs")
                    .write("code.c", "int main() { return 0; }")
                    .child(
                        commit("Rewrite specification")
                        .write("spec.txt", "Specification: Appear to do nothing")
                        .child(
                            commit("Add fancy logo")
                            .write("logo.txt", "[[[===Foo===]]]")
                            .child(
                                commit("Alternative logo")
                                .write("logo.txt", "***---Foo---***")
                                .branch("logo")
                                .child(
                                    commit("Tweak alternative logo")
                                    .write("logo.txt", "*---Foo---*")
                                    .branch("logo2")
                                )
                            )
                            .child(
                                commit("Rewrite in Rust")
                                .delete("code.c")
                                .write("code.rs", "fn main() { }")
                                .child(
                                    commit("Add hidden crypto mining code")
                                    .write("code.rs", "fn main() { start_mining_slave(); }")
                                    .child(
                                        commit("Tweak logo")
                                        .write("logo.txt", "[===Foo===]")
                                        .child(
                                            commit("Add motivational messages")
                                            .write("code.rs", "/* Don't slave away for your whole life. Win the lottery instead! */ fn main() { start_mining_slave(); }")
                                            .child(
                                                commit("Replace all instances of 'slave' with 'underling'")
                                                .write("code.rs", "/* Don't underling away for your whole life. Win the lottery instead! */ fn main() { start_mining_underling(); }")
                                                .branch("master")
                                            )
                                        )
                                    )
                                )
                            )
                        )
                    )
                )
                .child(
                    commit("Add readme (WIP)")
                    .write("Readme.md", "This is a project")
                    .child(
                        commit("More readme WIP")
                        .write("Readme.md", "This is a really great project")
                        .branch("readme")
                    )
                )
                .child(
                    commit("Make sweet asciinema demo")
                    .write("demo.asciinema", "...")
                    .branch("demo")
                )
            )
            .child(
                commit("Fix spelling")
                .write("spec.txt", "Specification: Do nothing")
                .branch("spelling")
            )
        );

    let repo = build_repo(&root, Some("master"));

    let repo_dir = repo.into_path();

    print_git_log_graph(&repo_dir);

    println!("Repo at: {:?}", repo_dir);
}
