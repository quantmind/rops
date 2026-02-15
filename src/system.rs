use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CurrentSystem {
    pub os: String,
    pub arch: String,
    pub arch_variant: Option<String>,
}

impl Default for CurrentSystem {
    fn default() -> Self {
        let (arch, arch_variant) = Self::get_arch();
        Self {
            os: std::env::consts::OS.to_string(),
            arch,
            arch_variant,
        }
    }
}

impl std::fmt::Display for CurrentSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.os, self.arch)?;
        Ok(())
    }
}

impl CurrentSystem {
    /// Derives the system architecture by executing `uname -m` and maps `x86_64` to `amd64`.
    fn get_arch() -> (String, Option<String>) {
        match Command::new("uname").arg("-m").output() {
            Ok(output) if output.status.success() => {
                let arch = String::from_utf8_lossy(&output.stdout).trim().to_string();
                match arch.as_str() {
                    "x86_64" => ("amd64".to_string(), Some(arch)),
                    "aarch64" => ("arm64".to_string(), Some(arch)),
                    _ => (arch, None),
                }
            }
            Ok(output) => {
                log::warn!(
                    "Failed to retrieve system architecture. Command exited with status: {}",
                    output.status
                );
                (String::new(), None)
            }
            Err(err) => {
                log::warn!("Failed to execute uname command: {}", err);
                (String::new(), None)
            }
        }
    }
}
