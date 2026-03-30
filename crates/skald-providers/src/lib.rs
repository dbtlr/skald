use async_trait::async_trait;

pub mod cli_provider;
pub mod config;

pub use cli_provider::CliProvider;
pub use config::{
    CliProviderConfig, available_provider_names, get_provider_config, is_provider_available,
};

#[derive(Debug, Clone)]
pub struct CommitContext {
    pub diff: String,
    pub stat: String,
    pub rendered_prompt: String,
    pub extra_context: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PrContext {
    pub diff: String,
    pub commit_log: String,
    pub target_branch: String,
    pub rendered_prompt: String,
    pub extra_context: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PrContent {
    pub title: String,
    pub body: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("Provider '{provider}' is not available: {detail}")]
    Unavailable { provider: String, detail: String },

    #[error("Provider '{provider}' returned an error: {detail}")]
    Generation { provider: String, detail: String },

    #[error("{0}")]
    Other(String),
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;

    async fn generate_commit_messages(
        &self,
        ctx: &CommitContext,
        count: usize,
    ) -> Result<Vec<String>, ProviderError>;

    async fn generate_pr_content(
        &self,
        ctx: &PrContext,
        count: usize,
    ) -> Result<Vec<PrContent>, ProviderError>;
}
