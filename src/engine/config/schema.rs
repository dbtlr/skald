use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct RawConfig {
    pub provider: Option<String>,
    pub language: Option<String>,
    pub pr_target: Option<String>,
    pub platform: Option<String>,
    pub vcs: Option<String>,
    pub providers: Option<HashMap<String, ProviderConfig>>,
    pub aliases: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ProviderConfig {
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub provider: String,
    pub language: String,
    pub pr_target: String,
    pub platform: String,
    pub vcs: String,
    pub providers: HashMap<String, ProviderConfig>,
    pub aliases: HashMap<String, String>,
    pub sources: HashMap<String, ConfigSource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    Default,
    Global,
    Project,
}

impl std::fmt::Display for ConfigSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Default => write!(f, "default"),
            Self::Global => write!(f, "global"),
            Self::Project => write!(f, "project"),
        }
    }
}
