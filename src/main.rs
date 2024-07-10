// Tool to automatically rebase branches.

use anyhow::Result;
use argh::FromArgs;

use autorebase::autorebase;

use std::env::current_dir;

#[derive(FromArgs)]
/// Automatically pull the master branch and rebase all branches without
/// upstreams onto it.
struct CliOptions {
    /// the target branch to pull and rebase onto;
    /// defaults to `git config --get init.defaultBranch` or `master` if unset
    #[argh(option)]
    onto: Option<String>,

    /// if there are conflicts, try rebasing commit by commit backwards from the
    /// target, instead of trying to determined the conflicting commit on the
    /// target branch directly
    #[argh(switch)]
    slow: bool,

    /// include branches which have an upstream, the default is to exclude these
    #[argh(switch)]
    include_non_local: bool,

    /// branch matching glob, the default is all branches
    #[argh(option)]
    match_branches: Option<String>,

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

    env_logger::Builder::new()
        .parse_filters(&options.log.unwrap_or_default())
        .init();

    autorebase(
        &current_dir()?,
        options.onto.as_deref(),
        options.slow,
        options.include_non_local,
        options.match_branches.as_deref(),
    )?;

    Ok(())
}
