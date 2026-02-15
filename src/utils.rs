use rand::TryRngCore;

use crate::error::{RopsError, RopsResult};
use std::io::{BufRead, BufReader};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};

/// A trait for types that can be created from an environment variable.
pub trait FromEnv: Sized {
    fn from_env(key: &str, default: Self) -> Self;
}

#[derive(Clone)]
pub struct Secret(String);

/// Implementation for simple String types.
impl FromEnv for String {
    fn from_env(key: &str, default: Self) -> Self {
        std::env::var(key).unwrap_or(default)
    }
}

/// Implementation for Option<String> types.
impl FromEnv for Option<String> {
    fn from_env(key: &str, default: Self) -> Self {
        match std::env::var(key) {
            Ok(val) => Some(val),
            Err(_) => default,
        }
    }
}

impl Secret {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() <= 3 {
            write!(f, "***")
        } else {
            let prefix = &self.0[..3];
            write!(f, "{}***", prefix)
        }
    }
}

impl serde::Serialize for Secret {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Secret(\"{}\")", self)
    }
}

/// Gets a value from an environment variable, returning a default if not present.
pub fn get_default_from_env<T: FromEnv>(key: &str, default: T) -> T {
    T::from_env(key, default)
}

pub fn home_bin(name: &str) -> RopsResult<std::path::PathBuf> {
    match std::env::home_dir() {
        Some(home) => Ok(home.join("bin").join(name)),
        None => Err(RopsError::Error("Failed to get home directory".into())),
    }
}

pub fn make_executable(path: &std::path::Path) -> RopsResult<()> {
    // Make the file executable on Unix-like systems
    #[cfg(unix)]
    {
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms)?;
    }
    Ok(())
}

pub fn as_true() -> bool {
    true
}

pub struct StreamCommand {
    pub command: Command,
    pub dry_run: bool,
    pub skip_error: Option<String>,
}

impl StreamCommand {
    pub fn new(command: Command) -> Self {
        Self {
            command,
            dry_run: false,
            skip_error: None,
        }
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    #[allow(dead_code)]
    pub fn skip_error<S: Into<String>>(mut self, error: S) -> Self {
        self.skip_error = Some(error.into());
        self
    }

    pub fn run(&mut self) -> RopsResult<bool> {
        log::info!("{}", self.format_command());
        if self.dry_run {
            log::info!("Dry run mode enabled, skipping actual command execution.");
            return Ok(true);
        }
        let mut child = self
            .command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        //
        let stdout = BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| RopsError::Error("Failed to capture stdout".into()))?,
        );
        let stderr = BufReader::new(
            child
                .stderr
                .take()
                .ok_or_else(|| RopsError::Error("Failed to capture stderr".into()))?,
        );
        let stdout_thread = std::thread::spawn(move || {
            for line in stdout.lines().map_while(Result::ok) {
                log::info!("{}", line);
            }
        });

        let maybe_skip_error = self.skip_error.clone();
        let stderr_thread = std::thread::spawn(move || {
            let mut error_lines = 0;
            for line in stderr.lines().map_while(Result::ok) {
                if let Some(ref skip_error) = maybe_skip_error
                    && line.contains(skip_error)
                {
                    log::debug!("skipping error: {}", line);
                    continue;
                }
                log::warn!("{}", line);
                error_lines += 1;
            }
            error_lines
        });

        let status = child.wait()?;
        stdout_thread
            .join()
            .map_err(|e| RopsError::Error(format!("Failed to join stdout thread: {:?}", e)))?;
        let error_lines = stderr_thread
            .join()
            .map_err(|e| RopsError::Error(format!("Failed to join stderr thread: {:?}", e)))?;
        if status.success() || error_lines == 0 {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn format_command(&self) -> String {
        let envs: Vec<String> = self
            .command
            .get_envs()
            .filter_map(|(k, v)| {
                v.map(|v_os| format!("{}={}", k.to_string_lossy(), v_os.to_string_lossy()))
            })
            .collect();

        let args: Vec<String> =
            std::iter::once(self.command.get_program().to_string_lossy().into_owned())
                .chain(
                    self.command
                        .get_args()
                        .map(|arg| arg.to_string_lossy().into_owned()),
                )
                .collect();

        if envs.is_empty() {
            args.join(" ")
        } else {
            format!("{} {}", envs.join(" "), args.join(" "))
        }
    }
}

pub fn rimraf(path: &str) -> RopsResult<()> {
    if std::path::Path::new(path).exists() {
        std::fs::remove_dir_all(path).map_err(|err| {
            RopsError::Error(format!("Failed to remove directory '{}': {}", path, err))
        })?;
    }
    Ok(())
}

pub fn random_base_64(length: usize) -> RopsResult<String> {
    use base64::{Engine as _, engine::general_purpose};
    let mut random_bytes = vec![0u8; length]; // Use a Vec<u8> for dynamic sizing
    let mut rng = rand::rngs::OsRng;
    rng.try_fill_bytes(&mut random_bytes)
        .map_err(|_| RopsError::Error("Failed to generate random bytes".into()))?;
    let encoded_string = general_purpose::URL_SAFE_NO_PAD.encode(&random_bytes);
    Ok(encoded_string)
}
