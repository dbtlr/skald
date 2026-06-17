use async_trait::async_trait;

pub mod anthropic;
pub mod cli_provider;
pub mod codex;
pub mod codex_auth;
pub mod config;
pub mod models;
pub mod resolve;
pub mod structured;

use crate::engine::config::schema::ResolvedConfig;
pub use anthropic::AnthropicProvider;
pub use cli_provider::CliProvider;

/// Create a provider instance based on provider name.
///
/// API providers (anthropic) use direct HTTP calls.
/// CLI providers (claude, codex, gemini, etc.) shell out to CLI binaries.
pub fn create_provider(
    provider_name: &str,
    model: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
    config: &ResolvedConfig,
) -> Result<Box<dyn Provider>, ProviderError> {
    if config::is_api_provider(provider_name) {
        // `codex` reuses the Codex CLI's ChatGPT-subscription login
        // (~/.codex/auth.json) instead of an API key, so it resolves before the
        // generic api-key path below. (The `codex-cli` provider is the separate
        // shell-out to the `codex` binary.)
        if provider_name == "codex" {
            let resolved_model = resolve::resolve_model(
                model.as_deref(),
                config,
                provider_name,
                codex::DEFAULT_CODEX_MODEL,
            );
            let resolved_url = resolve::resolve_base_url(
                base_url.as_deref(),
                config,
                provider_name,
                "CODEX_BASE_URL",
            );
            let provider =
                codex::CodexProvider::from_codex_auth(resolved_model, resolved_url).map_err(
                    |e| match e {
                        // The `codex` name now means the ChatGPT-subscription API
                        // path. Anyone who configured `codex` expecting the old CLI
                        // shell-out lands here — point them at `codex-cli`.
                        ProviderError::Unavailable { provider, detail } => {
                            ProviderError::Unavailable {
                                provider,
                                detail: format!(
                                    "{detail}\n\nNote: `codex` now uses your ChatGPT subscription directly. For the Codex CLI shell-out, use `--provider codex-cli`."
                                ),
                            }
                        }
                        other => other,
                    },
                )?;
            return Ok(Box::new(provider));
        }

        let resolved_key = resolve::resolve_api_key(
            api_key.as_deref(),
            config,
            provider_name,
            default_env_var(provider_name),
        )
        .map_err(|e| ProviderError::Unavailable {
            provider: provider_name.to_string(),
            detail: e,
        })?;

        let resolved_url = resolve::resolve_base_url(
            base_url.as_deref(),
            config,
            provider_name,
            default_base_url_env_var(provider_name),
        );

        let resolved_model = resolve::resolve_model(
            model.as_deref(),
            config,
            provider_name,
            default_model(provider_name),
        );

        match provider_name {
            "anthropic" => {
                let provider = AnthropicProvider::new(resolved_key, resolved_url, resolved_model);
                Ok(Box::new(provider))
            }
            _ => Err(ProviderError::Unavailable {
                provider: provider_name.to_string(),
                detail: format!("Unknown API provider '{provider_name}'"),
            }),
        }
    } else {
        let provider_config = config::get_provider_config(provider_name).ok_or_else(|| {
            // `codex-api` was renamed to `codex`; redirect rather than fail blankly.
            let hint = if provider_name == "codex-api" {
                " Did you mean `codex`? (The `codex-api` provider was renamed to `codex`.)"
            } else {
                ""
            };
            ProviderError::Unavailable {
                provider: provider_name.to_string(),
                detail: format!(
                    "Unknown provider '{}'. Available: {}.{hint}",
                    provider_name,
                    config::available_provider_names().join(", ")
                ),
            }
        })?;
        Ok(Box::new(CliProvider::new(provider_config, model)))
    }
}

fn default_env_var(provider_name: &str) -> &'static str {
    match provider_name {
        "anthropic" => "ANTHROPIC_API_KEY",
        _ => "",
    }
}

fn default_base_url_env_var(provider_name: &str) -> &'static str {
    match provider_name {
        "anthropic" => "ANTHROPIC_BASE_URL",
        _ => "",
    }
}

fn default_model(provider_name: &str) -> &'static str {
    match provider_name {
        "anthropic" => "claude-sonnet-4",
        _ => "",
    }
}

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
