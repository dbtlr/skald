use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelList {
    pub updated: String,
    pub providers: HashMap<String, ProviderModels>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModels {
    pub recommended: String,
    pub models: Vec<String>,
}

const FALLBACK_MODELS_JSON: &str = include_str!("../../../models.json");
const MODELS_URL: &str = "https://raw.githubusercontent.com/dbtlr/skald/main/models.json";
const FETCH_TIMEOUT: u64 = 3;
const CACHE_TTL: Duration = Duration::from_secs(86400);

pub fn fallback_models() -> ModelList {
    serde_json::from_str(FALLBACK_MODELS_JSON).expect("compiled-in models.json is invalid")
}

pub fn models_for_provider<'a>(list: &'a ModelList, provider: &str) -> Option<&'a ProviderModels> {
    list.providers.get(provider)
}

fn cache_path() -> std::path::PathBuf {
    let config_dir = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        std::path::PathBuf::from(xdg).join("skald")
    } else {
        dirs::home_dir()
            .map(|d| d.join(".config").join("skald"))
            .unwrap_or_else(|| std::path::PathBuf::from(".skald"))
    };
    config_dir.join("cache").join("models.json")
}

fn read_cache() -> Option<ModelList> {
    let path = cache_path();
    let metadata = std::fs::metadata(&path).ok()?;
    let modified = metadata.modified().ok()?;
    let age = SystemTime::now().duration_since(modified).ok()?;
    if age > CACHE_TTL {
        tracing::debug!("model cache is stale ({age:?} old), skipping");
        return None;
    }
    let contents = std::fs::read_to_string(&path).ok()?;
    match serde_json::from_str(&contents) {
        Ok(list) => {
            tracing::debug!("model cache hit: {}", path.display());
            Some(list)
        }
        Err(e) => {
            tracing::debug!("model cache parse error: {e}");
            None
        }
    }
}

fn write_cache(list: &ModelList) {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(list) {
        Ok(json) => {
            let _ = std::fs::write(&path, json);
            tracing::debug!("model cache written: {}", path.display());
        }
        Err(e) => {
            tracing::debug!("model cache write error: {e}");
        }
    }
}

fn fetch_remote() -> Option<ModelList> {
    tracing::debug!("fetching model list from {MODELS_URL}");
    let response = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(FETCH_TIMEOUT))
        .build()
        .get(MODELS_URL)
        .set("User-Agent", "skald-cli")
        .call()
        .ok()?;
    let body = response.into_string().ok()?;
    match serde_json::from_str(&body) {
        Ok(list) => {
            tracing::debug!("remote model list fetched successfully");
            Some(list)
        }
        Err(e) => {
            tracing::debug!("remote model list parse error: {e}");
            None
        }
    }
}

/// Resolve the model list: cache → remote → compiled-in fallback.
pub fn get_model_list() -> ModelList {
    if let Some(cached) = read_cache() {
        return cached;
    }
    if let Some(remote) = fetch_remote() {
        write_cache(&remote);
        return remote;
    }
    tracing::debug!("using compiled-in fallback model list");
    fallback_models()
}

/// Query `opencode models` at runtime and return the list of model IDs.
/// Returns None if opencode is not available or the command fails.
pub fn get_opencode_models() -> Option<Vec<String>> {
    let output = std::process::Command::new("opencode")
        .arg("models")
        .output()
        .ok()?;
    if !output.status.success() {
        tracing::debug!("opencode models command failed");
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let models: Vec<String> = stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    if models.is_empty() {
        None
    } else {
        tracing::debug!("opencode returned {} models", models.len());
        Some(models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_parses_successfully() {
        let list = fallback_models();
        assert!(!list.updated.is_empty());
    }

    #[test]
    fn all_four_providers_present() {
        let list = fallback_models();
        for provider in &["claude", "codex", "gemini", "copilot"] {
            assert!(
                list.providers.contains_key(*provider),
                "missing provider: {provider}"
            );
        }
    }

    #[test]
    fn each_provider_has_non_empty_recommended_in_models_list() {
        let list = fallback_models();
        for (name, provider) in &list.providers {
            assert!(
                !provider.recommended.is_empty(),
                "provider '{name}' has empty recommended"
            );
            assert!(
                provider.models.contains(&provider.recommended),
                "provider '{name}' recommended '{}' not in models list",
                provider.recommended
            );
            assert!(
                !provider.models.is_empty(),
                "provider '{name}' has empty models list"
            );
        }
    }

    #[test]
    fn lookup_known_provider() {
        let list = fallback_models();
        let result = models_for_provider(&list, "claude");
        assert!(result.is_some());
        assert_eq!(result.unwrap().recommended, "claude-haiku-4-5");
    }

    #[test]
    fn lookup_unknown_provider_returns_none() {
        let list = fallback_models();
        let result = models_for_provider(&list, "nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn raw_json_deserializes() {
        let raw = r#"{
            "updated": "2026-01-01",
            "providers": {
                "test": {
                    "recommended": "model-a",
                    "models": ["model-a", "model-b"]
                }
            }
        }"#;
        let list: ModelList = serde_json::from_str(raw).expect("should deserialize");
        assert_eq!(list.updated, "2026-01-01");
        let provider = list.providers.get("test").expect("test provider missing");
        assert_eq!(provider.recommended, "model-a");
        assert_eq!(provider.models, vec!["model-a", "model-b"]);
    }
}
