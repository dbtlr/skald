use crate::engine::config::schema::ResolvedConfig;
use crate::providers::models::resolve_model_alias;

/// Resolve the API key for an API provider.
///
/// Resolution chain: explicit value → config providers.<name>.api_key → default env var → error
pub fn resolve_api_key(
    explicit: Option<&str>,
    config: &ResolvedConfig,
    provider_name: &str,
    default_env_var: &str,
) -> Result<String, String> {
    // 1. Explicit value (from --api-key flag)
    if let Some(key) = explicit {
        if !key.is_empty() {
            return Ok(key.to_string());
        }
    }

    // 2. Config: providers.<name>.api_key (already env-expanded by config loader)
    if let Some(provider_cfg) = config.providers.get(provider_name) {
        if let Some(ref key) = provider_cfg.api_key {
            if !key.is_empty() {
                return Ok(key.clone());
            }
        }
    }

    // 3. Default env var for this provider
    if let Ok(key) = std::env::var(default_env_var) {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    Err(format!(
        "No API key found for '{provider_name}'.\n\n\
         Set via:\n  \
         --api-key flag\n  \
         providers.{provider_name}.api_key in config\n  \
         {default_env_var} environment variable"
    ))
}

/// Resolve the base URL for an API provider.
///
/// Resolution chain: explicit value → config providers.<name>.base_url → default env var → None
pub fn resolve_base_url(
    explicit: Option<&str>,
    config: &ResolvedConfig,
    provider_name: &str,
    default_env_var: &str,
) -> Option<String> {
    // 1. Explicit value (from --base-url flag)
    if let Some(url) = explicit {
        if !url.is_empty() {
            return Some(url.to_string());
        }
    }

    // 2. Config: providers.<name>.base_url (already env-expanded)
    if let Some(provider_cfg) = config.providers.get(provider_name) {
        if let Some(ref url) = provider_cfg.base_url {
            if !url.is_empty() {
                return Some(url.clone());
            }
        }
    }

    // 3. Default env var
    if let Ok(url) = std::env::var(default_env_var) {
        if !url.is_empty() {
            return Some(url);
        }
    }

    None
}

/// Resolve the model for an API provider.
///
/// Resolution chain: explicit value → config providers.<name>.model → default
/// All values are run through alias resolution.
pub fn resolve_model(
    explicit: Option<&str>,
    config: &ResolvedConfig,
    provider_name: &str,
    default_model: &str,
) -> String {
    let raw = explicit
        .map(|s| s.to_string())
        .or_else(|| {
            config
                .providers
                .get(provider_name)
                .and_then(|p| p.model.clone())
        })
        .unwrap_or_else(|| default_model.to_string());

    resolve_model_alias(&raw).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::config::schema::{ProviderConfig, ResolvedConfig};
    use std::collections::HashMap;

    fn empty_config() -> ResolvedConfig {
        ResolvedConfig {
            provider: "anthropic".to_string(),
            language: "English".to_string(),
            pr_target: "main".to_string(),
            platform: "github".to_string(),
            vcs: "git".to_string(),
            providers: HashMap::new(),
            aliases: HashMap::new(),
            sources: HashMap::new(),
        }
    }

    fn config_with_provider(api_key: Option<&str>, base_url: Option<&str>, model: Option<&str>) -> ResolvedConfig {
        let mut config = empty_config();
        config.providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                model: model.map(String::from),
                api_key: api_key.map(String::from),
                base_url: base_url.map(String::from),
            },
        );
        config
    }

    // --- API key tests ---

    #[test]
    fn api_key_from_explicit() {
        let config = empty_config();
        let result = resolve_api_key(Some("sk-explicit"), &config, "anthropic", "ANTHROPIC_API_KEY");
        assert_eq!(result.unwrap(), "sk-explicit");
    }

    #[test]
    fn api_key_from_config() {
        let config = config_with_provider(Some("sk-config"), None, None);
        let result = resolve_api_key(None, &config, "anthropic", "ANTHROPIC_API_KEY");
        assert_eq!(result.unwrap(), "sk-config");
    }

    #[test]
    fn api_key_explicit_overrides_config() {
        let config = config_with_provider(Some("sk-config"), None, None);
        let result = resolve_api_key(Some("sk-explicit"), &config, "anthropic", "ANTHROPIC_API_KEY");
        assert_eq!(result.unwrap(), "sk-explicit");
    }

    #[test]
    fn api_key_missing_returns_error() {
        let config = empty_config();
        let result = resolve_api_key(None, &config, "anthropic", "SKALD_TEST_NONEXISTENT_KEY_12345");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("No API key found"));
        assert!(err.contains("--api-key flag"));
        assert!(err.contains("providers.anthropic.api_key"));
    }

    // --- Base URL tests ---

    #[test]
    fn base_url_from_explicit() {
        let config = empty_config();
        let result = resolve_base_url(Some("https://custom.api"), &config, "anthropic", "ANTHROPIC_BASE_URL");
        assert_eq!(result.unwrap(), "https://custom.api");
    }

    #[test]
    fn base_url_from_config() {
        let config = config_with_provider(None, Some("https://config.api"), None);
        let result = resolve_base_url(None, &config, "anthropic", "ANTHROPIC_BASE_URL");
        assert_eq!(result.unwrap(), "https://config.api");
    }

    #[test]
    fn base_url_none_when_missing() {
        let config = empty_config();
        let result = resolve_base_url(None, &config, "anthropic", "SKALD_TEST_NONEXISTENT_URL_12345");
        assert!(result.is_none());
    }

    // --- Model tests ---

    #[test]
    fn model_from_explicit() {
        let config = empty_config();
        let result = resolve_model(Some("opus"), &config, "anthropic", "claude-sonnet-4");
        assert_eq!(result, "claude-opus-4");
    }

    #[test]
    fn model_from_config() {
        let config = config_with_provider(None, None, Some("haiku"));
        let result = resolve_model(None, &config, "anthropic", "claude-sonnet-4");
        assert_eq!(result, "claude-haiku-4-5");
    }

    #[test]
    fn model_default_when_missing() {
        let config = empty_config();
        let result = resolve_model(None, &config, "anthropic", "claude-sonnet-4");
        assert_eq!(result, "claude-sonnet-4");
    }

    #[test]
    fn model_passthrough_full_id() {
        let config = empty_config();
        let result = resolve_model(Some("claude-sonnet-4-20250514"), &config, "anthropic", "claude-sonnet-4");
        assert_eq!(result, "claude-sonnet-4-20250514");
    }
}
