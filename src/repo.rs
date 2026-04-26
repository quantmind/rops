use crate::{
    error::{RopsError, RopsResult},
    settings::Settings,
};
use semver::Version;
use std::{fs, path::Path, process::Command, thread, time::Duration};

#[derive(clap::Subcommand, Debug, Clone)]
pub enum RepoCommand {
    /// Show information about the repo
    Info,
    /// Update version
    UpdateVersion {
        /// The version to update to
        version: String,
    },
}

impl RepoCommand {
    /// Run the Docker command
    pub fn run(&self, settings: &Settings) -> RopsResult<()> {
        match self {
            RepoCommand::Info => {
                println!("Git Branch: {}", settings.git.branch);
                println!("Git SHA: {}", settings.git.sha);
                println!(
                    "Docker Image Repo URL: {}",
                    settings.get_repo_url("example")
                );
                Ok(())
            }
            RepoCommand::UpdateVersion { version } => self.update_version(settings, version),
        }
    }

    pub fn update_version(&self, settings: &Settings, new_version: &str) -> RopsResult<()> {
        // 1. Validate the version string
        let parsed_version = Version::parse(new_version)?;

        // 2. Update the version in all TOML files listed in `self.toml`
        for toml_file in &settings.project.toml {
            if !Path::new(toml_file).exists() {
                return Err(RopsError::GitError(format!(
                    "TOML file '{}' does not exist",
                    toml_file
                )));
            }

            // Read the TOML file
            let content = fs::read_to_string(toml_file).map_err(|err| {
                RopsError::TomlError(format!("Failed to read TOML file '{}': {}", toml_file, err))
            })?;

            let updated_content =
                set_toml_version(&content, &parsed_version.to_string()).map_err(|_| {
                    RopsError::TomlError(format!(
                        "No 'version' field found in [package], [project], or [workspace.package] section of '{}'",
                        toml_file
                    ))
                })?;

            // Write the updated content back to the file
            fs::write(toml_file, updated_content)
                .map_err(|err| format!("Failed to write TOML file '{}': {}", toml_file, err))?;
        }

        thread::sleep(Duration::from_secs(2));

        let output = Command::new("git")
            .arg("commit")
            .arg("-a")
            .arg("-m")
            .arg(format!("Bump version to v{}", new_version))
            .output()
            .map_err(|err| RopsError::GitError(format!("Failed to execute git commit: {}", err)))?;
        if !output.status.success() {
            return Err(RopsError::GitError(format!(
                "Git commit failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        println!("Changes committed successfully.");

        let output = Command::new("git")
            .arg("tag")
            .arg(format!("v{}", new_version))
            .output()
            .map_err(|err| {
                RopsError::GitError(format!("Failed to execute git command: {}", err))
            })?;
        if output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        } else {
            return Err(RopsError::GitError(format!(
                "Git tag creation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // 5. Push the tag to the remote repository
        let output = Command::new("git")
            .arg("push")
            .arg("--tags")
            .output()
            .map_err(|err| RopsError::GitError(format!("Failed to execute git push: {}", err)))?;
        if output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
            println!("tag {} pushed successfully", new_version);
        } else {
            return Err(RopsError::GitError(format!(
                "Git push failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        Ok(())
    }
}

/// Updates the `version` field inside a `[package]`, `[project]`, or
/// `[workspace.package]` section of TOML source text.  Returns the modified
/// content on success, or `Err(())` if no matching version line was found.
pub fn set_toml_version(content: &str, new_version: &str) -> Result<String, ()> {
    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let mut in_target_section = false;
    let mut updated = false;

    for line in &mut lines {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_target_section =
                trimmed == "[package]" || trimmed == "[project]" || trimmed == "[workspace.package]";
        }
        if in_target_section && trimmed.starts_with("version") {
            *line = format!("version = \"{}\"", new_version);
            updated = true;
            break;
        }
    }

    if updated {
        Ok(lines.join("\n"))
    } else {
        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::set_toml_version;

    fn make_toml(section: &str, version: &str) -> String {
        format!("[{}]\nname = \"my-crate\"\nversion = \"{}\"\n", section, version)
    }

    #[test]
    fn updates_package_section() {
        let input = make_toml("package", "0.1.0");
        let result = set_toml_version(&input, "1.2.3").unwrap();
        assert!(result.contains("version = \"1.2.3\""));
    }

    #[test]
    fn updates_project_section() {
        let input = make_toml("project", "0.1.0");
        let result = set_toml_version(&input, "2.0.0").unwrap();
        assert!(result.contains("version = \"2.0.0\""));
    }

    #[test]
    fn updates_workspace_package_section() {
        let input = make_toml("workspace.package", "0.1.0");
        let result = set_toml_version(&input, "3.4.5").unwrap();
        assert!(result.contains("version = \"3.4.5\""));
    }

    #[test]
    fn ignores_version_outside_target_section() {
        let input = "[dependencies]\nfoo = { version = \"1.0\" }\n\n[package]\nversion = \"0.1.0\"\n";
        let result = set_toml_version(input, "9.9.9").unwrap();
        assert!(result.contains("[package]\nversion = \"9.9.9\""));
        assert!(result.contains("foo = { version = \"1.0\" }"));
    }

    #[test]
    fn returns_err_when_no_version_found() {
        let input = "[package]\nname = \"my-crate\"\n";
        assert!(set_toml_version(input, "1.0.0").is_err());
    }

    #[test]
    fn returns_err_for_unknown_section() {
        let input = "[lib]\nversion = \"0.1.0\"\n";
        assert!(set_toml_version(input, "1.0.0").is_err());
    }
}
