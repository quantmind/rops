use crate::{
    error::{RopsError, RopsResult},
    settings::Settings,
    utils::{Secret, StreamCommand, rimraf},
};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct GitSettings {
    #[serde(default = "GitSettings::get_default_branch")]
    pub default_branch: String,
    #[serde(default = "GitSettings::get_git_branch")]
    pub branch: String,
    #[serde(default = "GitSettings::get_git_sha")]
    pub sha: String,
    #[serde(default = "GitSettings::get_github_token", skip_deserializing)]
    pub github_token: Option<Secret>,
}

#[derive(Clone, Debug)]
pub struct GithubDownloadRelease {
    pub repo: String,
    pub file_name: String,
    pub client: Client,
    pub token: Option<Secret>,
    pub version: Option<String>,
    /// A different download url
    pub download_url: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub version: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Release {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

impl GitSettings {
    pub fn is_default_branch(&self) -> bool {
        self.branch == self.default_branch
    }

    fn get_default_branch() -> String {
        std::env::var("GIT_DEFAULT_BRANCH").unwrap_or_else(|_| "main".to_string())
    }

    pub fn get_github_token() -> Option<Secret> {
        std::env::var("GITHUB_TOKEN").ok().map(Secret::new)
    }

    /// Derives the Git SHA by executing `git rev-parse HEAD`.
    fn get_git_sha() -> String {
        match Command::new("git").arg("rev-parse").arg("HEAD").output() {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            }
            Ok(output) => {
                log::warn!(
                    "Failed to retrieve Git SHA. Git command exited with status: {}",
                    output.status
                );
                String::new()
            }
            Err(err) => {
                log::warn!("Failed to execute Git command: {}", err);
                String::new()
            }
        }
    }

    /// Derives the Git branch by checking the environment variable in CodeBuild or executing `git rev-parse --abbrev-ref HEAD`.
    fn get_git_branch() -> String {
        // Try to get the branch name using `git symbolic-ref HEAD --short`
        let output = Command::new("git")
            .arg("symbolic-ref")
            .arg("HEAD")
            .arg("--short")
            .output();

        if let Ok(output) = output
            && output.status.success()
        {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !branch.is_empty() {
                return branch;
            }
        }

        // Fallback: Use `git branch -a --contains HEAD` and process the output
        let output = Command::new("git")
            .arg("branch")
            .arg("-a")
            .arg("--contains")
            .arg("HEAD")
            .output();

        if let Ok(output) = output
            && output.status.success()
        {
            let branch_output = String::from_utf8_lossy(&output.stdout);
            if let Some(branch_line) = branch_output.lines().nth(1) {
                let branch = branch_line
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();
                return branch.trim_start_matches("remotes/origin/").to_string();
            }
        }

        // If all else fails, return "unknown"
        log::warn!("Unable to determine Git branch name. Returning 'unknown'.");
        "".to_string()
    }

    pub fn clone_repo(repo_name: &str, repo: &str) -> RopsResult<()> {
        rimraf(repo_name)?;
        let mut child = Command::new("git");
        child.arg("clone").arg(repo).arg(repo_name);
        if StreamCommand::new(child).run()? {
            Ok(())
        } else {
            Err(RopsError::Error(format!(
                "Failed to clone Git repo '{}' into '{}'",
                repo, repo_name
            )))
        }
    }

    pub fn release_downloader(&self, repo: &str, file_name: &str) -> GithubDownloadRelease {
        GithubDownloadRelease::new(repo, file_name, self.github_token.clone())
    }
}

impl GithubDownloadRelease {
    pub fn new(repo: &str, file_name: &str, token: Option<Secret>) -> Self {
        Self {
            repo: repo.to_string(),
            file_name: file_name.to_string(),
            client: Client::new(),
            token,
            version: None,
            download_url: None,
        }
    }

    pub fn with_version<S: Into<String>>(mut self, version: S) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn with_download_url<S: Into<String>>(mut self, download_url: S) -> Self {
        self.download_url = Some(download_url.into());
        self
    }

    pub fn request(&self, url: String) -> reqwest::blocking::RequestBuilder {
        let mut builder = self.client.get(url).header("User-Agent", "quantmind/rops");
        if let Some(ref token) = self.token {
            builder = builder.header("Authorization", format!("Bearer {}", token.value()));
        }
        builder
    }

    pub fn get_release(&self, _settings: &Settings) -> RopsResult<Release> {
        let url = if let Some(version) = &self.version {
            let url = format!(
                "https://api.github.com/repos/{}/releases/tags/{}",
                self.repo, version
            );
            log::info!("Fetching release {} information from GitHub {url}", version);
            url
        } else {
            let url = format!("https://api.github.com/repos/{}/releases/latest", self.repo);
            log::info!("Fetching latest release information from GitHub {url}");
            url
        };
        // Fetch the latest release information from GitHub
        let release: Release = self
            .request(url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .map_err(|err| RopsError::Error(err.to_string()))?
            .json()
            .map_err(|err| RopsError::Error(err.to_string()))?;
        Ok(release)
    }

    pub fn get_asset(&self, settings: &Settings) -> RopsResult<Asset> {
        let release = self.get_release(settings)?;
        let mut file_names = vec![];
        for arch in [
            Some(&settings.system.arch),
            settings.system.arch_variant.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            let file_name = self.get_file_name(settings, &release, arch);
            file_names.push(file_name.clone());
            if let Some(download_url) = &self.download_url {
                return Ok(Asset {
                    url: format!("{download_url}/{file_name}"),
                    name: file_name,
                    version: release.tag_name.clone(),
                });
            } else {
                // Find the asset matching the current architecture
                if let Some(asset) = release
                    .assets
                    .iter()
                    .find(|a| a.name.to_lowercase() == file_name)
                {
                    return Ok(Asset {
                        url: asset.url.clone(),
                        name: asset.name.clone(),
                        version: release.tag_name.clone(),
                    });
                }
            }
        }
        Err(RopsError::Error(format!(
            "Asset '{}' not found in release",
            file_names.join("', '")
        )))
    }

    pub fn get_file_name(&self, settings: &Settings, release: &Release, arch: &str) -> String {
        self.file_name
            .replace("{version}", &release.tag_name)
            .replace("{os}", &settings.system.os)
            .replace("{arch}", arch)
    }

    pub fn download(&self, settings: &Settings) -> RopsResult<Asset> {
        let asset = self.get_asset(settings)?;

        log::info!(
            "Download version {} - {} from {}",
            asset.version,
            asset.name,
            asset.url
        );

        // Download the binary using the asset's API URL
        let mut response = self
            .request(asset.url.clone())
            .header("Accept", "application/octet-stream")
            .send()
            .map_err(|err| RopsError::Error(err.to_string()))?;

        if !response.status().is_success() {
            return Err(RopsError::Error(format!(
                "Download failed with status: {}",
                response.status()
            )));
        }
        response
            .copy_to(&mut std::fs::File::create(&asset.name)?)
            .map_err(|err| RopsError::Error(err.to_string()))?;
        Ok(asset)
    }
}
