use std::collections::HashMap;
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

pub fn fallback_models() -> ModelList {
    serde_json::from_str(FALLBACK_MODELS_JSON).expect("compiled-in models.json is invalid")
}

pub fn models_for_provider<'a>(list: &'a ModelList, provider: &str) -> Option<&'a ProviderModels> {
    list.providers.get(provider)
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
