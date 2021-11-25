use std::path::Path;

use anyhow::{anyhow, Error};
use git2::{build::RepoBuilder, Direction, IntoCString, Repository};

pub(crate) fn update_repo(path: &Path) -> Result<(), Error> {
    let repo = Repository::open(path)?;

    let mut remote = repo.find_remote("origin")?;
    remote.connect(Direction::Fetch)?;
    let branch = remote.default_branch()?.into_c_string()?;
    let branch = branch.to_str()?;
    remote.fetch(&[branch], None, None)?;
    remote.disconnect()?;
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let analysis = repo.merge_analysis(&[&fetch_commit])?;
    if analysis.0.is_up_to_date() {
        println!("Repo up to date");
        Ok(())
    } else if analysis.0.is_fast_forward() {
        let refname = format!("refs/heads/{}", branch);
        let mut reference = repo.find_reference(&refname)?;
        reference.set_target(fetch_commit.id(), "Fast-Forward")?;
        repo.set_head(&refname)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
            .map_err(Error::from)
    } else {
        Err(anyhow!("Fast-forward only!"))
    }
}
pub(crate) fn clone_repo(path: &Path, url: &str) -> Result<(), Error> {
    let mut builder = RepoBuilder::new();
    // TODO: use these to report progress properly
    // let mut fetch_opts = FetchOptions::new();
    // let mut remote_callbacks = RemoteCallbacks::new();
    // remote_callbacks.transfer_progress(|progress| {});
    // fetch_opts.remote_callbacks(remote_callbacks);
    // builder.fetch_options(fetch_opts);
    builder.clone(url, path)?;
    Ok(())
}
