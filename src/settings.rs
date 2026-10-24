use super::{blocks, charts, docker, git, system};
use crate::{error::RopsResult, utils::new_table};
use comfy_table::Table;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

    /// Render the settings as a table of dotted keys and values
    pub fn table(&self) -> RopsResult<Table> {
        let mut rows = Vec::new();
        flatten_value("", &serde_json::to_value(self)?, &mut rows);
        let mut table = new_table(["Key", "Value"]);
        for row in rows {
            table.add_row(row);
        }
        Ok(table)
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

/// Flatten a JSON value into `(dotted.key, value)` rows
fn flatten_value(prefix: &str, value: &Value, rows: &mut Vec<[String; 2]>) {
    let key = |k: &str| {
        if prefix.is_empty() {
            k.to_string()
        } else {
            format!("{prefix}.{k}")
        }
    };
    match value {
        Value::Object(map) if !map.is_empty() => {
            for (k, v) in map {
                flatten_value(&key(k), v, rows);
            }
        }
        Value::Array(items) if items.iter().any(|v| v.is_object() || v.is_array()) => {
            for (i, v) in items.iter().enumerate() {
                flatten_value(&key(&i.to_string()), v, rows);
            }
        }
        _ => rows.push([prefix.to_string(), scalar_to_string(value)]),
    }
}

fn scalar_to_string(value: &Value) -> String {
    match value {
        Value::Null => "-".to_string(),
        Value::String(s) if s.is_empty() => "-".to_string(),
        Value::String(s) => s.clone(),
        Value::Array(items) if items.is_empty() => "-".to_string(),
        Value::Array(items) => items
            .iter()
            .map(scalar_to_string)
            .collect::<Vec<_>>()
            .join(", "),
        Value::Object(_) => "-".to_string(),
        other => other.to_string(),
    }
}
