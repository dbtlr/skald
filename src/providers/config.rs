use std::process::Command;

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
    let mut names: Vec<&str> = ALL_PROVIDERS.iter().map(|p| p.name).collect();
    names.extend_from_slice(API_PROVIDERS);
    names
}

pub fn is_provider_available(name: &str) -> bool {
    match get_provider_config(name) {
        Some(config) => Command::new("which")
            .arg(config.binary)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false),
        // API providers aren't binaries in PATH — detect them by whether usable
        // credentials are present right now.
        None => api_provider_has_credentials(name),
    }
}

/// Whether an API provider has usable credentials available right now, used by
/// `config init` to decide which providers to offer.
///
/// `codex` rides the Codex ChatGPT-subscription login (`~/.codex/auth.json`);
/// `anthropic` uses an API key from the environment.
fn api_provider_has_credentials(name: &str) -> bool {
    match name {
        "codex" => crate::providers::codex_auth::load_codex_creds().is_ok(),
        "anthropic" => std::env::var("ANTHROPIC_API_KEY").is_ok_and(|v| !v.is_empty()),
        _ => false,
    }
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
    fn is_provider_available_does_not_panic() {
        // Just verify it doesn't panic for CLI, API, and unknown providers
        let _ = is_provider_available("claude");
        let _ = is_provider_available("codex-cli");
        let _ = is_provider_available("gemini");
        let _ = is_provider_available("opencode");
        let _ = is_provider_available("copilot");
        let _ = is_provider_available("codex");
        let _ = is_provider_available("anthropic");
        let _ = is_provider_available("unknown");
        let _ = is_provider_available("");
    }

    #[test]
    fn api_provider_has_credentials_rejects_non_api_names() {
        assert!(!api_provider_has_credentials("claude"));
        assert!(!api_provider_has_credentials("codex-cli"));
        assert!(!api_provider_has_credentials("unknown"));
    }
}
