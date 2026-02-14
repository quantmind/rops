use crate::{
    error::{RopsError, RopsResult},
    settings::Settings,
    utils::as_true,
};
use reqwest::{Method, blocking::Client};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BlockSettings {
    #[serde(default = "Metablock::get_default_api_url")]
    pub api_url: String,
    #[serde(default = "Metablock::get_default_space")]
    pub default_space: String,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct BlockConfig {
    pub name: String,
    pub space: Option<String>,
    pub upstream: String,
    pub routes: Vec<Route>,
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub root: bool,
    #[serde(default)]
    pub html: bool,
    #[serde(default)]
    pub used_cdn: bool,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Route {
    pub name: String,
    pub protocols: Vec<String>,
    pub paths: Vec<String>,
    #[serde(default)]
    pub plugins: Vec<Plugin>,
    #[serde(default = "as_true")]
    pub preserve_host: bool,
    #[serde(default)]
    pub strip_path: bool,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Plugin {
    pub name: String,
    pub config: serde_json::Value, // Use serde_json::Value for flexible plugin configuration
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Space {
    pub id: String,
    pub name: String,
    pub hosted: bool,
    pub domain: String,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Block {
    pub id: String,
    pub name: String,
    pub space: Space,
    pub full_name: String,
}

pub struct Metablock {
    pub api_url: String,
    pub api_token: String,
    pub client: Client,
}

impl Default for BlockSettings {
    fn default() -> Self {
        Self {
            api_url: Metablock::get_default_api_url(),
            default_space: Metablock::get_default_space(),
        }
    }
}

impl BlockSettings {
    pub fn metablock(&self) -> RopsResult<Metablock> {
        let api_token = std::env::var("METABLOCK_API_TOKEN").map_err(|_| {
            RopsError::Error(
                "METABLOCK_API_TOKEN not set - add it to your env or the .env file".into(),
            )
        })?;
        Ok(Metablock::new(&self.api_url, api_token))
    }
}

impl Metablock {
    pub fn get_default_api_url() -> String {
        std::env::var("METABLOCK_API_URL")
            .unwrap_or_else(|_| "https://api.metablock.io".to_string())
    }

    pub fn get_default_space() -> String {
        std::env::var("METABLOCK_SPACE").unwrap_or_else(|_| "metablock".to_string())
    }

    pub fn new<S1: Into<String>, S2: Into<String>>(api_url: S1, api_token: S2) -> Self {
        Self {
            api_url: api_url.into(),
            api_token: api_token.into(),
            client: Client::new(),
        }
    }

    pub fn request(&self, method: Method, url: String) -> reqwest::blocking::RequestBuilder {
        self.client
            .request(method, url)
            .header("User-Agent", "quantmind/rops")
            .header("x-metablock-api-key", &self.api_token)
    }

    pub fn apply(&self, settings: &Settings, block_config: &BlockConfig) -> RopsResult<()> {
        let space_name = block_config
            .space
            .clone()
            .unwrap_or_else(|| settings.blocks.default_space.clone());
        if let Some(block) = self.get_block(&space_name, &block_config.name)? {
            log::info!(
                "Block '{}' already exists in space '{space_name}'. Updating...",
                block_config.name,
            );
            let block = self.update_block(&block.id, block_config)?;
            log::info!("Block '{}' updated", block.full_name);
        } else {
            log::info!(
                "Creating new block '{}' in space '{space_name}'",
                block_config.name,
            );
            let block = self.create_block(&space_name, block_config)?;
            log::info!("Block '{}' created", block.full_name);
        }
        Ok(())
    }

    pub fn get_block(&self, space_name: &str, block_name: &str) -> RopsResult<Option<Block>> {
        let url = format!(
            "{}/v1/spaces/{space_name}/blocks?name={block_name}",
            self.api_url
        );
        log::info!("Fetching block information from {url}");
        let blocks: Vec<Block> = self.request(Method::GET, url).send()?.json()?;
        if blocks.is_empty() {
            Ok(None)
        } else {
            Ok(Some(blocks[0].clone()))
        }
    }

    pub fn create_block(&self, space_name: &str, block_config: &BlockConfig) -> RopsResult<Block> {
        let url = format!("{}/v1/spaces/{space_name}/blocks", self.api_url);
        let response = self.request(Method::POST, url).json(block_config).send()?;
        if response.status().is_client_error() {
            return Err(RopsError::Error(format!(
                "Failed to create block - status {}: {}",
                response.status(),
                response.text()?
            )));
        }
        Ok(response.json()?)
    }

    pub fn update_block(&self, block_id: &str, block_config: &BlockConfig) -> RopsResult<Block> {
        let url = format!("{}/v1/blocks/{block_id}", self.api_url);
        let response = self.request(Method::PATCH, url).json(block_config).send()?;
        if response.status().is_client_error() {
            return Err(RopsError::Error(format!(
                "Failed to update block - status {}: {}",
                response.status(),
                response.text()?
            )));
        }
        Ok(response.json()?)
    }
}
