use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{anyhow, Error};

pub(crate) fn update_repo(path: &Path) -> Result<(), Error> {
    Command::new("git")
        .args(["pull"])
        .current_dir(path)
        .stdout(Stdio::inherit())
        .spawn()?
        .wait()?
        .success()
        .then(|| ())
        .ok_or(anyhow!("Git error!"))
}
pub(crate) fn clone_repo(path: &Path, url: &str) -> Result<(), Error> {
    Command::new("git")
        .args(["clone", url, "."])
        .current_dir(path)
        .spawn()?
        .wait()?
        .success()
        .then(|| ())
        .ok_or(anyhow!("Git error!"))
}
