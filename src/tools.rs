use super::settings::Settings;
use crate::{
    error::{RopsError, RopsResult},
    git::GithubDownloadRelease,
    utils,
};
use std::collections::HashMap;

#[derive(clap::Subcommand, Debug, Clone)]
pub enum ToolsCommand {
    /// List all configured third party tools
    #[command(alias = "ls")]
    List,
    /// Update a third party tool
    Update {
        /// The name of the tool to update
        tool: String,
        /// Specify the version to target
        #[arg(short, long)]
        version: Option<String>,
    },
}

struct Tools {
    tools: HashMap<String, ThirdPartyTool>, // tool name and version
}

enum InstallMethod {
    GithubDownload(GithubDownloadRelease),
}

struct ThirdPartyTool {
    name: String,
    description: String,
    method: InstallMethod,
}

impl ToolsCommand {
    /// Run the Docker command
    pub fn run(&self, settings: &Settings) -> RopsResult<()> {
        let tools = Tools::default();
        match self {
            Self::List => {
                let mut tool_list: Vec<_> = tools.tools.iter().collect();
                tool_list.sort_by_key(|(name, _)| *name);
                for (name, tool) in tool_list {
                    println!("{}: {}", name, tool.description);
                }
                Ok(())
            }
            Self::Update { tool, version } => {
                if let Some(tool) = tools.tools.get(tool) {
                    tool.update(settings, version.as_deref())
                } else {
                    Err(RopsError::Error(format!("Tool {} not found", tool)))
                }
            }
        }
    }
}

impl Default for Tools {
    fn default() -> Self {
        Self {
            tools: vec![
                ThirdPartyTool::new(
                    "helm",
                    "The Kubernetes Package Manager",
                    InstallMethod::GithubDownload(
                        GithubDownloadRelease::new(
                            "helm/helm",
                            "helm-{version}-{os}-{arch}.tar.gz",
                        )
                        .with_download_url("https://get.helm.sh"),
                    ),
                ),
                ThirdPartyTool::new(
                    "k9s",
                    "K9s is a terminal based UI to interact with your Kubernetes clusters",
                    InstallMethod::GithubDownload(GithubDownloadRelease::new(
                        "derailed/k9s",
                        "k9s_{os}_{arch}.tar.gz",
                    )),
                ),
                ThirdPartyTool::new(
                    "taplo",
                    "Configuration file editor for YAML and TOML",
                    InstallMethod::GithubDownload(GithubDownloadRelease::new(
                        "tamasfe/taplo",
                        "taplo-{os}-{arch}.gz",
                    )),
                ),
                ThirdPartyTool::new(
                    "sops",
                    "Secrets management tool",
                    InstallMethod::GithubDownload(GithubDownloadRelease::new(
                        "getsops/sops",
                        "sops-{version}.{os}.{arch}",
                    )),
                ),
            ]
            .into_iter()
            .map(|tool| (tool.name.clone(), tool))
            .collect(),
        }
    }
}

impl ThirdPartyTool {
    fn new(name: &str, description: &str, method: InstallMethod) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            method,
        }
    }

    fn update(&self, settings: &Settings, version: Option<&str>) -> RopsResult<()> {
        match &self.method {
            InstallMethod::GithubDownload(g) => {
                let mut g = g.clone();
                if let Some(version) = version {
                    g = g.with_version(version);
                }
                let target = utils::home_bin(&self.name)?;
                let asset = g.download(settings)?;
                self.move_to_target(&asset.name, target.as_path())?;
                // Remove the downloaded archive/file
                std::fs::remove_file(&asset.name)?;
                utils::make_executable(target.as_path())?;
                log::info!("Updated {} to version {}", self.name, asset.version);
                Ok(())
            }
        }
    }

    fn move_to_target(&self, file_name: &str, target: &std::path::Path) -> RopsResult<()> {
        if file_name.ends_with(".gz") {
            log::info!("Extracting from .gz archive {file_name}...");
            let file = std::fs::File::open(file_name)?;
            let mut decoder = flate2::read::GzDecoder::new(file);
            if file_name.ends_with(".tar.gz") {
                log::info!("Extracting from tar archive {file_name}...");
                let mut archive = tar::Archive::new(decoder);
                // Find the binary in the archive and extract it to the target path.
                let mut entry_found = false;
                for entry in archive.entries()? {
                    let mut entry = entry?;
                    if entry.path()?.ends_with(&self.name) {
                        entry.unpack(target)?;
                        entry_found = true;
                        break;
                    }
                }
                if !entry_found {
                    return Err(RopsError::Error(format!(
                        "Could not find binary '{}' in the archive {}",
                        self.name, file_name
                    )));
                }
            } else {
                // decode a single file
                let mut target_file = std::fs::File::create(target)?;
                std::io::copy(&mut decoder, &mut target_file)?;
            }
        } else {
            std::fs::copy(file_name, target).map_err(|err| {
                RopsError::Error(format!("Failed to copy {} to {target:?}: {err}", file_name))
            })?;
        }
        Ok(())
    }
}
