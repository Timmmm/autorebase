// Tool to automatically rebase branches.

use anyhow::Result;
use argh::FromArgs;

use autorebase::autorebase;

use std::env::current_dir;

#[derive(FromArgs)]
/// Automatically pull the master branch and rebase all branches without
/// upstreams onto it.
struct CliOptions {
    /// the target branch to pull and rebase onto (typically "master" or "develop")
    #[argh(option, default = "String::from(\"master\")")]
    onto: String,

    /// if there are conflicts, try rebasing commit by commit backwards from the
    /// target, instead of trying to determind the conflicting commit on the
    /// target branch directly
    #[argh(switch)]
    slow: bool,

    /// include branches which have an upstream, the default is to exclude these
    #[argh(switch)]
    all_branches: bool,

    /// RUST_LOG-style logging string, e.g. --log debug
    #[argh(option)]
    log: Option<String>,
}

fn main() -> Result<()> {
    let res = run();
    if res.is_err() {
        // Print a newline because there may be a half finished output
        // (e.g. using `eprint!()` instead of `eprintln!()`.
        eprintln!();
    }
    res
}

fn run() -> Result<()> {
    let options: CliOptions = argh::from_env();

    env_logger::Builder::new().parse_filters(&options.log.unwrap_or_default()).init();

    autorebase(
        &current_dir()?,
        &options.onto,
        options.slow,
        options.all_branches,
    )?;

    Ok(())
}
