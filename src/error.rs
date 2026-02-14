use thiserror::Error;

pub type RopsResult<T> = Result<T, RopsError>;

#[derive(Error, Debug)]
pub enum RopsError {
    #[error("{0}")]
    DockerError(String),
    #[error("{0}")]
    GitError(String),
    #[error("{0}")]
    TomlError(String),
    #[error(transparent)]
    VersionError(#[from] semver::Error),
    #[error("{0}")]
    Error(String),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    SerdeYamlError(#[from] serde_yaml::Error),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
}

impl From<String> for RopsError {
    fn from(error: String) -> Self {
        Self::Error(error)
    }
}
impl From<&str> for RopsError {
    fn from(error: &str) -> Self {
        Self::Error(error.to_string())
    }
}
