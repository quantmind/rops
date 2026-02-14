use super::{blocks, charts, docker, git, system};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use toml::from_str;

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Project {
    #[serde(default)]
    pub toml: Vec<String>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Settings {
    #[serde(default)]
    pub system: system::CurrentSystem,
    pub git: git::GitSettings,
    pub docker: docker::DockerSettings,
    pub project: Project,
    #[serde(default)]
    pub charts: charts::ChartsSettings,
    #[serde(default)]
    pub blocks: blocks::BlockSettings,
}

impl Settings {
    pub fn get_repo_name(&self, name: &str) -> String {
        match &self.docker.image_prefix {
            Some(prefix) => format!("{}-{}", prefix, name),
            None => name.to_string(),
        }
    }

    pub fn get_repo_url(&self, name: &str) -> String {
        format!(
            "{}/{}",
            self.docker.image_repo_url,
            self.get_repo_name(name)
        )
    }

    pub fn get_git_tag(&self) -> String {
        if self.git.is_default_branch() {
            format!("{}-{}", self.git.branch, self.git.sha)
        } else {
            format!("{}-{}", self.docker.image_branch_tag_prefix, self.git.sha)
        }
    }

    pub fn load(config_path: &str) -> Self {
        if Path::new(config_path).exists() {
            match fs::read_to_string(config_path) {
                Ok(content) => match from_str::<Settings>(&content) {
                    Ok(settings) => settings,
                    Err(err) => {
                        log::error!("Failed to parse configuration: {}", err);
                        Self::default()
                    }
                },
                Err(err) => {
                    log::error!("Failed to read configuration file: {}", err);
                    Self::default()
                }
            }
        } else {
            log::warn!("Configuration file not found: {}", config_path);
            Self::default()
        }
    }
}
