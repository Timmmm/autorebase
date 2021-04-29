use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use anyhow::Result;
use std::fs;
use std::path::Path;

// Store information about which branches failed to rebase due to conflicts
// so we don't keep wasting time retrying. It's basically a list of branches
// that failed rebasing due to conflicts, and the hash of the branch.
// We need the user to manually rebase them (which will change the hash).
//
// It's not perfect since if they are just working on that branch without rebasing
// it we will keep retrying the rebase, but it'll do.

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Conflicts {
    /// Map from branch to the commit it pointed to when it got stuck due to conflicts.
    pub branches: HashMap<String, String>,
}

impl Conflicts {
    pub fn read_from_file(path: &Path) -> Result<Self> {
        let s = fs::read_to_string(path)?;
        let c = toml::from_str(&s)?;
        Ok(c)
    }

    pub fn write_to_file(&self, path: &Path) -> Result<()> {
        let s = toml::to_string(&self)?;
        fs::write(path, s)?;
        Ok(())
    }
}
