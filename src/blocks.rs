use crate::{
    error::{RopsError, RopsResult},
    settings::Settings,
    utils::as_true,
};
use reqwest::{
    Method,
    blocking::{Client, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BlockSettings {
    #[serde(default = "Metablock::get_default_api_url")]
    pub api_url: String,
    #[serde(default = "Metablock::get_default_space")]
    pub default_space: String,
    /// Organization the metablock API calls act within, by name or by id
    ///
    /// It is the organization owning the spaces the blocks belong to, which is
    /// not necessarily named after them: the `quantmind` space is owned by the
    /// `metablock` organization.
    #[serde(default = "Metablock::get_default_org")]
    pub org: String,
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
pub struct Org {
    pub id: String,
    pub short_name: String,
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
    /// Id of the organization the requests act within
    pub org_id: String,
    pub client: Client,
}

impl Default for BlockSettings {
    fn default() -> Self {
        Self {
            api_url: Metablock::get_default_api_url(),
            default_space: Metablock::get_default_space(),
            org: Metablock::get_default_org(),
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
        Metablock::new(&self.api_url, api_token).with_org(&self.org)
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

    pub fn get_default_org() -> String {
        std::env::var("METABLOCK_ORG").unwrap_or_else(|_| "metablock".to_string())
    }

    fn new<S1: Into<String>, S2: Into<String>>(api_url: S1, api_token: S2) -> Self {
        Self {
            api_url: api_url.into(),
            api_token: api_token.into(),
            org_id: String::new(),
            client: Client::new(),
        }
    }

    /// Resolve the organization the requests act within
    ///
    /// The header only matches organizations by id, so an organization named
    /// in the configuration has to be looked up first.
    fn with_org(mut self, org: &str) -> RopsResult<Self> {
        let url = format!("{}/v1/orgs/{org}", self.api_url);
        log::info!("Fetching organization information from {url}");
        let org: Org = check(self.key_request(Method::GET, url).send()?)?.json()?;
        log::info!(
            "Acting within organization '{}' - {}",
            org.short_name,
            org.id
        );
        self.org_id = org.id;
        Ok(self)
    }

    /// A request authenticated with the API key only
    ///
    /// Used for the endpoints resolving the organization itself, which cannot
    /// require the organization header.
    fn key_request(&self, method: Method, url: String) -> reqwest::blocking::RequestBuilder {
        self.client
            .request(method, url)
            .header("User-Agent", "quantmind/rops")
            .header("x-metablock-api-key", &self.api_token)
    }

    /// A request acting within the resolved organization
    ///
    /// The API resolves the organization from this header and answers `422`
    /// when it is missing, so every block endpoint needs it.
    pub fn request(&self, method: Method, url: String) -> reqwest::blocking::RequestBuilder {
        self.key_request(method, url)
            .header("x-metablock-org-id", &self.org_id)
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
        let blocks: Vec<Block> = check(self.request(Method::GET, url).send()?)?.json()?;
        if blocks.is_empty() {
            Ok(None)
        } else {
            Ok(Some(blocks[0].clone()))
        }
    }

    pub fn create_block(&self, space_name: &str, block_config: &BlockConfig) -> RopsResult<Block> {
        let url = format!("{}/v1/spaces/{space_name}/blocks", self.api_url);
        let response = self.request(Method::POST, url).json(block_config).send()?;
        Ok(check(response)?.json()?)
    }

    pub fn update_block(&self, block_id: &str, block_config: &BlockConfig) -> RopsResult<Block> {
        let url = format!("{}/v1/blocks/{block_id}", self.api_url);
        let response = self.request(Method::PATCH, url).json(block_config).send()?;
        Ok(check(response)?.json()?)
    }
}

/// Report an error response by its status and body
///
/// Decoding an error response would fail with reqwest's opaque "error decoding
/// response body", losing both the status and the message from the API.
fn check(response: Response) -> RopsResult<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let url = response.url().to_string();
    Err(RopsError::Error(format!(
        "{status} from {url}: {}",
        response.text()?
    )))
}

#[cfg(test)]
mod tests {
    use super::{Block, Org};

    // Responses from the live API, the fields the structs do not use removed

    #[test]
    fn org_is_deserialized() {
        let org: Org = serde_json::from_str(
            r#"{"email":"admin@metablock.io","short_name":"metablock","full_name":"",
                "status":"created","id":"63b97d659eb7487c9cb287d68fcbb38a",
                "created":"2025-04-10T09:55:39.494335Z","additional_info":{}}"#,
        )
        .unwrap();
        assert_eq!(org.short_name, "metablock");
        assert_eq!(org.id, "63b97d659eb7487c9cb287d68fcbb38a");
    }

    #[test]
    fn block_list_is_deserialized() {
        let blocks: Vec<Block> = serde_json::from_str(
            r#"[{"id":"2a9a109290da48d9ae611793d812e5e6",
                 "service_id":"b66516e371dd488d909cb55cb41cfcb6","name":"code",
                 "space":{"cdn":"","hosted":true,"name":"quantmind",
                          "domain":"quantmind.com","id":"1904c2c1b2304672a98df556d4773f27",
                          "org_id":"63b97d659eb7487c9cb287d68fcbb38a","org_name":""},
                 "full_name":"code-quantmind","html":false,"root":false,"acme":true,
                 "domain":"code.quantmind.com","url":"https://code.quantmind.com"}]"#,
        )
        .unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].full_name, "code-quantmind");
        assert_eq!(blocks[0].space.name, "quantmind");
    }
}
