use std::process::Command;

use crate::engine::config::schema::ResolvedConfig;

#[derive(Debug, Clone)]
pub struct CliProviderConfig {
    pub name: &'static str,
    pub binary: &'static str,
    pub prompt_args: &'static [&'static str],
    pub tool_args: &'static [&'static str],
    pub model_flag: &'static str,
}

static CLAUDE: CliProviderConfig = CliProviderConfig {
    name: "claude",
    binary: "claude",
    prompt_args: &["-p"],
    tool_args: &["--allowedTools", "Read"],
    model_flag: "--model",
};

static CODEX_CLI: CliProviderConfig = CliProviderConfig {
    name: "codex-cli",
    binary: "codex",
    prompt_args: &["exec"],
    tool_args: &["--sandbox", "read-only"],
    model_flag: "-m",
};

static GEMINI: CliProviderConfig = CliProviderConfig {
    name: "gemini",
    binary: "gemini",
    prompt_args: &["-p"],
    tool_args: &["--yolo"],
    model_flag: "-m",
};

static OPENCODE: CliProviderConfig = CliProviderConfig {
    name: "opencode",
    binary: "opencode",
    prompt_args: &["run"],
    tool_args: &["--allowedTools", "read"],
    model_flag: "-m",
};

static COPILOT: CliProviderConfig = CliProviderConfig {
    name: "copilot",
    binary: "copilot",
    prompt_args: &["-p"],
    tool_args: &["--allow-all-tools"],
    model_flag: "--model",
};

static ALL_PROVIDERS: &[&CliProviderConfig] = &[&CLAUDE, &CODEX_CLI, &GEMINI, &OPENCODE, &COPILOT];

/// API providers (direct HTTP, not CLI wrappers).
///
/// `codex` reuses the Codex CLI's ChatGPT-subscription login
/// (`~/.codex/auth.json`) instead of an API key. The `codex-cli` provider is
/// the separate shell-out to the `codex` binary.
const API_PROVIDERS: &[&str] = &["anthropic", "codex"];

/// Check if a provider name is a known API provider (not a CLI wrapper).
pub fn is_api_provider(name: &str) -> bool {
    API_PROVIDERS.contains(&name)
}

pub fn get_provider_config(name: &str) -> Option<&'static CliProviderConfig> {
    ALL_PROVIDERS.iter().copied().find(|p| p.name == name)
}

pub fn available_provider_names() -> Vec<&'static str> {
    // API (direct HTTP) providers come first — they're the recommended paths and
    // should surface ahead of the CLI shell-out providers in `config init` and in
    // suggestions.
    let mut names: Vec<&str> = API_PROVIDERS.to_vec();
    names.extend(ALL_PROVIDERS.iter().map(|p| p.name));
    names
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderReadiness {
    Ready,
    NeedsSetup,
    NotInstalled,
}

/// Whether a CLI binary resolves on the PATH.
fn binary_in_path(binary: &str) -> bool {
    Command::new("which").arg(binary).output().map(|o| o.status.success()).unwrap_or(false)
}

/// Advisory readiness of a provider for `config init` — a label, never a gate.
///
/// CLI providers are `Ready` when their binary is on the PATH, else
/// `NotInstalled`. API providers are `Ready` when credentials resolve via the
/// same chain the runtime uses (`can_resolve_api_key` / the codex auth loader),
/// else `NeedsSetup`.
pub fn provider_readiness(name: &str, config: Option<&ResolvedConfig>) -> ProviderReadiness {
    if let Some(cfg) = get_provider_config(name) {
        return if binary_in_path(cfg.binary) {
            ProviderReadiness::Ready
        } else {
            ProviderReadiness::NotInstalled
        };
    }
    if is_api_provider(name) {
        let has_creds = match name {
            "codex" => crate::providers::codex_auth::load_codex_creds().is_ok(),
            _ => crate::providers::resolve::can_resolve_api_key(
                config,
                name,
                crate::providers::default_env_var(name),
            ),
        };
        return if has_creds { ProviderReadiness::Ready } else { ProviderReadiness::NeedsSetup };
    }
    ProviderReadiness::NotInstalled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_five_providers_resolve() {
        for name in ["claude", "codex-cli", "gemini", "opencode", "copilot"] {
            assert!(get_provider_config(name).is_some(), "provider '{name}' should resolve");
        }
    }

    #[test]
    fn unknown_provider_returns_none() {
        assert!(get_provider_config("unknown-provider").is_none());
        assert!(get_provider_config("").is_none());
    }

    #[test]
    fn available_provider_names_lists_all() {
        let names = available_provider_names();
        assert_eq!(names.len(), 7);
        assert!(names.contains(&"claude"));
        assert!(names.contains(&"codex-cli"));
        assert!(names.contains(&"gemini"));
        assert!(names.contains(&"opencode"));
        assert!(names.contains(&"copilot"));
        assert!(names.contains(&"anthropic"));
        assert!(names.contains(&"codex"));
    }

    #[test]
    fn is_api_provider_identifies_api_providers() {
        assert!(is_api_provider("anthropic"));
        assert!(is_api_provider("codex"));
        assert!(!is_api_provider("claude"));
        assert!(!is_api_provider("codex-cli"));
        assert!(!is_api_provider("unknown"));
    }

    #[test]
    fn claude_config_has_correct_fields() {
        let config = get_provider_config("claude").unwrap();
        assert_eq!(config.name, "claude");
        assert_eq!(config.binary, "claude");
        assert_eq!(config.prompt_args, &["-p"]);
        assert_eq!(config.tool_args, &["--allowedTools", "Read"]);
        assert_eq!(config.model_flag, "--model");
    }

    #[test]
    fn codex_config_has_correct_fields() {
        let config = get_provider_config("codex-cli").unwrap();
        assert_eq!(config.name, "codex-cli");
        assert_eq!(config.binary, "codex");
        assert_eq!(config.prompt_args, &["exec"]);
        assert_eq!(config.tool_args, &["--sandbox", "read-only"]);
        assert_eq!(config.model_flag, "-m");
    }

    #[test]
    fn gemini_config_has_correct_fields() {
        let config = get_provider_config("gemini").unwrap();
        assert_eq!(config.name, "gemini");
        assert_eq!(config.binary, "gemini");
        assert_eq!(config.prompt_args, &["-p"]);
        assert_eq!(config.tool_args, &["--yolo"]);
        assert_eq!(config.model_flag, "-m");
    }

    #[test]
    fn opencode_config_has_correct_fields() {
        let config = get_provider_config("opencode").unwrap();
        assert_eq!(config.name, "opencode");
        assert_eq!(config.binary, "opencode");
        assert_eq!(config.prompt_args, &["run"]);
        assert_eq!(config.tool_args, &["--allowedTools", "read"]);
        assert_eq!(config.model_flag, "-m");
    }

    #[test]
    fn copilot_config_has_correct_fields() {
        let config = get_provider_config("copilot").unwrap();
        assert_eq!(config.name, "copilot");
        assert_eq!(config.binary, "copilot");
        assert_eq!(config.prompt_args, &["-p"]);
        assert_eq!(config.tool_args, &["--allow-all-tools"]);
        assert_eq!(config.model_flag, "--model");
    }

    #[test]
    fn provider_readiness_does_not_panic() {
        for name in [
            "claude",
            "codex-cli",
            "gemini",
            "opencode",
            "copilot",
            "codex",
            "anthropic",
            "unknown",
            "",
        ] {
            let _ = provider_readiness(name, None);
        }
    }

    #[test]
    fn anthropic_ready_with_config_key() {
        use crate::engine::config::schema::{ProviderConfig, ResolvedConfig};
        use std::collections::HashMap;

        let mut providers = HashMap::new();
        providers.insert(
            "anthropic".to_string(),
            ProviderConfig { model: None, api_key: Some("sk-x".to_string()), base_url: None },
        );
        let config = ResolvedConfig {
            provider: "anthropic".to_string(),
            language: "English".to_string(),
            pr_target: "main".to_string(),
            platform: "github".to_string(),
            vcs: "git".to_string(),
            providers,
            aliases: HashMap::new(),
            sources: HashMap::new(),
        };

        assert_eq!(provider_readiness("anthropic", Some(&config)), ProviderReadiness::Ready);
    }
}
