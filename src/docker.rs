use crate::settings::Settings;
use crate::{
    error::{RopsError, RopsResult},
    utils::{StreamCommand, get_default_from_env},
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct DockerSettings {
    #[serde(default = "_default_docker_files_path")]
    pub files_path: String,
    #[serde(default = "_default_docker_image_prefix")]
    pub image_prefix: Option<String>,
    #[serde(default = "_default_docker_image_branch_tag_prefix")]
    pub image_branch_tag_prefix: String,
    #[serde(default = "_default_docker_image_repo_url")]
    pub image_repo_url: String,
    #[serde(default = "_default_docker_git_sha_arg")]
    pub git_sha_arg: Option<String>,
}

fn _default_docker_files_path() -> String {
    get_default_from_env("DOCKER_FILES_PATH", "".into())
}
fn _default_docker_image_prefix() -> Option<String> {
    get_default_from_env("DOCKER_IMAGE_PREFIX", None)
}
fn _default_docker_image_branch_tag_prefix() -> String {
    get_default_from_env("DOCKER_IMAGE_BRANCH_TAG_PREFIX", "branch".into())
}
fn _default_docker_image_repo_url() -> String {
    get_default_from_env("DOCKER_IMAGE_REPO_URL", "".into())
}
fn _default_docker_git_sha_arg() -> Option<String> {
    get_default_from_env("DOCKER_GIT_SHA_ARG", None)
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum DockerCommand {
    /// Build a new Docker image
    Build {
        /// Image name
        name: String,
        /// Path to the Dockerfile
        #[arg(short, long)]
        dockerfile: Option<String>,
        /// tag with image repo url
        #[arg(long, action = clap::ArgAction::SetTrue)]
        tag_url: bool,
        /// Build arguments
        #[arg(short, long, num_args = 1..)]
        build_args: Vec<String>,
    },
    /// Push a Docker image to a registry
    Push {
        /// Image name
        name: String,
        /// Add architecture suffix to image tag (e.g. -amd64, -arm64)
        #[arg(long, action = clap::ArgAction::SetTrue)]
        arch: bool,
    },
    /// Create and push a Docker manifest
    Manifest {
        /// Image name
        name: String,
    },
}

impl DockerCommand {
    /// Run the Docker command
    pub fn run(&self, settings: &Settings) -> RopsResult<()> {
        match self {
            Self::Build {
                name,
                dockerfile,
                build_args,
                tag_url,
            } => {
                let dockerfile = self.get_dockerfile(name, dockerfile, settings);
                let image_name = settings.get_repo_name(name);
                let mut build_args = build_args.clone();
                // Add the git sha arg if settings is set
                if let Some(git_sha_arg) = &settings.docker.git_sha_arg {
                    build_args.push(format!("{}={}", git_sha_arg, settings.git.sha));
                }

                // Prepare the Docker build command
                let mut command = Command::new("docker");
                command
                    .env("DOCKER_BUILDKIT", "1") // Enable Docker BuildKit
                    .arg("build")
                    .arg("-f")
                    .arg(&dockerfile)
                    .arg("--platform")
                    .arg(settings.system.to_string())
                    .arg("-t")
                    .arg(&image_name);

                if *tag_url {
                    command.arg("-t").arg(settings.get_repo_url(name));
                }

                command.arg("."); // Build context

                // Add build arguments
                for arg in build_args {
                    command.arg("--build-arg").arg(arg);
                }

                if StreamCommand::new(command).run()? {
                    Ok(())
                } else {
                    Err(RopsError::DockerError("Docker build failed".to_string()))
                }
            }
            Self::Push { name, arch } => {
                let image_name = settings.get_repo_name(name);
                let tag = self.get_push_tag(name, *arch, settings);

                let mut command = Command::new("docker");
                command
                    .env("DOCKER_BUILDKIT", "1") // Enable Docker BuildKit
                    .arg("tag")
                    .arg(&image_name) // Correct image name
                    .arg(&tag);

                if !StreamCommand::new(command).run()? {
                    return Err(RopsError::DockerError(format!(
                        "Docker tag failed for {}",
                        tag
                    )));
                }

                // Push all tags with --all-tags flag
                let mut command = Command::new("docker");
                command
                    .env("DOCKER_BUILDKIT", "1") // Enable Docker BuildKit
                    .arg("push")
                    .arg(&tag);

                if StreamCommand::new(command).run()? {
                    Ok(())
                } else {
                    Err(RopsError::DockerError("Docker push failed".to_string()))
                }
            }
            Self::Manifest { name } => {
                let manifest_tag = self.get_push_tag(name, false, settings);
                let amd64_tag = format!("{}-amd64", manifest_tag);
                let arm64_tag = format!("{}-arm64", manifest_tag);
                self.push_manifest(&manifest_tag, &amd64_tag, &arm64_tag)?;
                if let Some(latest_tag) = self.get_latest_tag(name, settings) {
                    self.push_manifest(&latest_tag, &amd64_tag, &arm64_tag)?;
                }
                Ok(())
            }
        }
    }

    fn push_manifest(
        &self,
        manifest_tag: &str,
        amd64_tag: &str,
        arm64_tag: &str,
    ) -> RopsResult<()> {
        // Create the manifest
        let mut manifest_create = Command::new("docker");
        manifest_create
            .arg("manifest")
            .arg("create")
            .arg("-a")
            .arg(manifest_tag)
            .arg(amd64_tag)
            .arg(arm64_tag);

        if !StreamCommand::new(manifest_create).run()? {
            return Err(RopsError::DockerError(
                "Docker manifest create failed".to_string(),
            ));
        }

        // Annotate the manifest for amd64
        let mut manifest_annotate_amd64 = Command::new("docker");
        manifest_annotate_amd64
            .arg("manifest")
            .arg("annotate")
            .arg(manifest_tag)
            .arg(amd64_tag)
            .arg("--os")
            .arg("linux")
            .arg("--arch")
            .arg("amd64");

        if !StreamCommand::new(manifest_annotate_amd64).run()? {
            return Err(RopsError::DockerError(
                "Docker manifest annotate for amd64 failed".to_string(),
            ));
        }

        // Annotate the manifest for arm64
        let mut manifest_annotate_arm64 = Command::new("docker");
        manifest_annotate_arm64
            .arg("manifest")
            .arg("annotate")
            .arg(manifest_tag)
            .arg(arm64_tag)
            .arg("--os")
            .arg("linux")
            .arg("--arch")
            .arg("arm64");

        if !StreamCommand::new(manifest_annotate_arm64).run()? {
            return Err(RopsError::DockerError(
                "Docker manifest annotate for arm64 failed".to_string(),
            ));
        }

        // Push the manifest
        let mut manifest_push = Command::new("docker");
        manifest_push.arg("manifest").arg("push").arg(manifest_tag);

        if StreamCommand::new(manifest_push).run()? {
            log::info!("Docker manifest pushed successfully: {}", manifest_tag);
            Ok(())
        } else {
            Err(RopsError::DockerError(
                "Docker manifest push failed".to_string(),
            ))
        }
    }

    /// Get the Dockerfile path
    fn get_dockerfile(
        &self,
        name: &str,
        dockerfile: &Option<String>,
        settings: &Settings,
    ) -> String {
        dockerfile.clone().unwrap_or_else(|| {
            let mut path = Path::new(&settings.docker.files_path).join(name);
            path.set_extension("dockerfile");
            path.to_string_lossy().to_string()
        })
    }

    fn get_push_tag(&self, name: &str, arch: bool, settings: &Settings) -> String {
        let repo_url = settings.get_repo_url(name);
        let arch_suffix = if arch {
            format!("-{}", settings.system.arch)
        } else {
            String::new()
        };
        format!("{}:{}{}", repo_url, settings.get_git_tag(), arch_suffix)
    }

    fn get_latest_tag(&self, name: &str, settings: &Settings) -> Option<String> {
        if settings.git.is_default_branch() {
            let repo_url = settings.get_repo_url(name);
            return Some(format!("{}:latest", repo_url));
        }
        None
    }
}
