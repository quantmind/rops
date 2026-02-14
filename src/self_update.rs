use crate::git;
use std::env;

use crate::{
    error::{RopsError, RopsResult},
    settings::Settings,
};

pub fn self_update(settings: &Settings) -> RopsResult<()> {
    let github_token = env::var("GITHUB_TOKEN").map_err(|_| {
        RopsError::Error("GITHUB_TOKEN not set - add it to the .env file".to_string())
    })?;
    let installer =
        git::GithubDownloadRelease::new("quantmind/devops", "rops-{arch}").with_token(github_token);
    let asset = installer.download(settings)?;

    self_replace::self_replace(&asset.name).map_err(|err| RopsError::Error(err.to_string()))?;
    std::fs::remove_file(&asset.name)?;
    log::info!("Self-update completed successfully.");
    Ok(())
}
