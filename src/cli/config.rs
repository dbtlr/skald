use crate::engine::config::{ResolvedConfig, global_config_path};
use crate::engine::output::OutputFormat;
use crate::providers::config::{
    ProviderReadiness, available_provider_names, get_provider_config, is_api_provider,
    provider_readiness,
};
use crate::providers::models::{get_model_list, get_opencode_models, models_for_provider};

/// Short readiness label shown beside a provider in the picker / listing.
fn readiness_hint(r: ProviderReadiness) -> &'static str {
    match r {
        ProviderReadiness::Ready => "ready",
        ProviderReadiness::NeedsSetup => "needs setup",
        ProviderReadiness::NotInstalled => "not installed",
    }
}

/// Guidance printed after selecting a non-ready provider. `None` when ready.
fn next_step_hint(name: &str, r: ProviderReadiness) -> Option<String> {
    match r {
        ProviderReadiness::Ready => None,
        ProviderReadiness::NeedsSetup if name == "codex" => Some(
            "Run `codex login` to authorize your ChatGPT subscription (creates ~/.codex/auth.json)."
                .to_string(),
        ),
        ProviderReadiness::NeedsSetup => Some(format!(
            "Add credentials for '{name}': set the provider's API key via environment variable, or providers.{name}.api_key in your config."
        )),
        ProviderReadiness::NotInstalled => {
            Some(format!("Install the '{name}' CLI and ensure it is on your PATH."))
        }
    }
}

fn build_config_template(provider: &str, model: Option<&str>) -> String {
    let model_section = match model {
        Some(m) => {
            format!("\n# Provider-specific settings\nproviders:\n  {provider}:\n    model: {m}\n")
        }
        None => format!(
            "\n# Provider-specific settings\n# providers:\n#   {provider}:\n#     model: <model-name>\n"
        ),
    };

    format!(
        r#"# Skald configuration
# See: https://github.com/dbtlr/skald/docs/configuration.md

# AI provider (default: claude)
provider: {provider}

# Language for generated messages (default: English)
# language: English

# Default PR target branch (default: main)
# pr_target: main
{model_section}
# Aliases — composable flag shortcuts
# aliases:
#   ci: "commit -n 5"
#   ca: "commit --auto -A"
#   fix: "commit --auto -a --context 'bug fix'"
"#
    )
}

fn write_config(provider: &str, model: Option<&str>) -> i32 {
    let path = global_config_path();

    if path.exists() {
        cliclack::log::info(format!("Config already exists at {}", path.display())).ok();
        return 0;
    }

    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        cliclack::log::error(format!("Failed to create directory {}: {e}", parent.display())).ok();
        return 1;
    }

    let content = build_config_template(provider, model);
    if let Err(e) = std::fs::write(&path, &content) {
        cliclack::log::error(format!("Failed to write config: {e}")).ok();
        return 1;
    }

    cliclack::log::success(format!("Config created at {}", path.display())).ok();
    0
}

fn pick_opencode_model() -> Option<String> {
    let models = match get_opencode_models() {
        Some(m) if !m.is_empty() => m,
        _ => {
            cliclack::log::info("Could not query OpenCode models.").ok();
            let input: Result<String, _> = cliclack::input("Model (optional):")
                .placeholder("Enter a model name or leave blank")
                .required(false)
                .interact();
            return match input {
                Ok(m) if !m.is_empty() => Some(m),
                _ => None,
            };
        }
    };

    let mut select = cliclack::select("Select a model:");
    for model in &models {
        select = select.item(model.clone(), model, "");
    }
    select = select.item("__other__".to_string(), "Other (enter manually)", "");

    match select.interact() {
        Ok(choice) if choice == "__other__" => {
            let input: Result<String, _> =
                cliclack::input("Model:").placeholder("Enter model name").interact();
            match input {
                Ok(m) if !m.is_empty() => Some(m),
                _ => None,
            }
        }
        Ok(choice) => Some(choice),
        Err(_) => None,
    }
}

fn resolve_init_model(model_arg: Option<&str>, provider: &str, is_tty: bool) -> Option<String> {
    if let Some(model) = model_arg {
        return Some(model.to_string());
    }

    if !is_tty {
        return None;
    }

    if provider == "opencode" {
        return pick_opencode_model();
    }

    let model_list = get_model_list();
    let provider_models = match models_for_provider(&model_list, provider) {
        Some(m) => m,
        None => {
            // No models known — manual input
            let input: Result<String, _> = cliclack::input("Model (optional):")
                .placeholder("Enter a model name or leave blank for default")
                .required(false)
                .interact();
            return match input {
                Ok(m) if !m.is_empty() => Some(m),
                _ => None,
            };
        }
    };

    let mut select = cliclack::select("Select a model:");
    select = select.item(
        provider_models.recommended.clone(),
        &provider_models.recommended,
        "recommended",
    );
    for model in &provider_models.models {
        if model != &provider_models.recommended {
            select = select.item(model.clone(), model, "");
        }
    }
    select = select.item("__other__".to_string(), "Other (enter manually)", "");

    match select.interact() {
        Ok(choice) if choice == "__other__" => {
            let input: Result<String, _> =
                cliclack::input("Model:").placeholder("Enter model name").interact();
            match input {
                Ok(m) if !m.is_empty() => Some(m),
                _ => None,
            }
        }
        Ok(choice) => Some(choice),
        Err(_) => {
            cliclack::log::info("Skipped model selection.").ok();
            None
        }
    }
}

pub fn run_init(
    provider_arg: Option<&str>,
    model_arg: Option<&str>,
    is_tty: bool,
    config: Option<&ResolvedConfig>,
) -> i32 {
    // With --provider flag: validate, report readiness, write (intent honored).
    if let Some(provider) = provider_arg {
        if get_provider_config(provider).is_none() && !is_api_provider(provider) {
            let known = available_provider_names().join(", ");
            cliclack::log::error(format!(
                "Unknown provider '{provider}'. Known providers: {known}"
            ))
            .ok();
            return 1;
        }
        let readiness = provider_readiness(provider, config);
        if readiness != ProviderReadiness::Ready {
            cliclack::log::warning(format!(
                "Provider '{provider}' is {}. Config will be written anyway.",
                readiness_hint(readiness)
            ))
            .ok();
        }
        let code = write_config(provider, model_arg);
        if code == 0
            && let Some(hint) = next_step_hint(provider, readiness)
        {
            cliclack::log::info(hint).ok();
        }
        return code;
    }

    let all_names = available_provider_names();
    let readiness: Vec<(&str, ProviderReadiness)> =
        all_names.iter().map(|&name| (name, provider_readiness(name, config))).collect();

    // Non-interactive: list every provider with its readiness, suggest a command.
    if !is_tty {
        eprintln!("error: No provider specified. Skald needs an AI provider to work.");
        eprintln!();
        eprintln!("Providers:");
        for (name, r) in &readiness {
            eprintln!("  {name:<12} ({})", readiness_hint(*r));
        }
        eprintln!();
        if let Some((first_ready, _)) =
            readiness.iter().find(|(_, r)| *r == ProviderReadiness::Ready)
        {
            eprintln!("Run: sk config init --provider {first_ready}");
        } else {
            eprintln!("No providers are ready yet. Pick one and set it up:");
            eprintln!("  sk config init --provider anthropic   # then set ANTHROPIC_API_KEY");
            eprintln!("  claude: https://claude.ai/download");
            eprintln!("  codex:  https://github.com/openai/codex");
        }
        return 1;
    }

    // Interactive: offer every provider, labeled with readiness. Selection = intent.
    let provider_options: Vec<(&str, &str, &'static str)> =
        readiness.iter().map(|&(name, r)| (name, name, readiness_hint(r))).collect();

    let selected_provider =
        match cliclack::select("Select an AI provider").items(&provider_options).interact() {
            Ok(p) => p,
            Err(_) => return 1,
        };

    let model = resolve_init_model(model_arg, selected_provider, is_tty);
    let code = write_config(selected_provider, model.as_deref());
    if code == 0 {
        let selected_readiness = provider_readiness(selected_provider, config);
        if let Some(hint) = next_step_hint(selected_provider, selected_readiness) {
            cliclack::log::info(hint).ok();
        }
    }
    code
}

pub fn run_eject(project: bool, name: Option<&str>) -> i32 {
    let target_dir = if project {
        std::env::current_dir().unwrap_or_default().join(".skald").join("prompts")
    } else {
        crate::engine::config::config_dir().join("prompts")
    };

    let names: Option<Vec<&str>> = name.map(|n| vec![n]);
    let names_ref = names.as_deref();

    match crate::engine::prompts::eject_prompts(&target_dir, names_ref) {
        Ok(written) => {
            if written.is_empty() {
                cliclack::log::info(format!(
                    "All templates already exist in {}",
                    target_dir.display()
                ))
                .ok();
            } else {
                for name in &written {
                    cliclack::log::success(format!(
                        "Ejected {name}.md → {}",
                        target_dir.join(format!("{name}.md")).display()
                    ))
                    .ok();
                }
            }
            0
        }
        Err(e) => {
            cliclack::log::error(e.to_string()).ok();
            1
        }
    }
}

pub fn run_show(config: &ResolvedConfig, format: OutputFormat, is_tty: bool) -> i32 {
    let headers = vec!["Key", "Value", "Source"];

    let model = config
        .providers
        .get(&config.provider)
        .and_then(|p| p.model.as_deref())
        .unwrap_or("(default)");

    let source_for = |key: &str| -> String {
        config.sources.get(key).map(|s| s.to_string()).unwrap_or_else(|| "default".to_string())
    };

    let rows = vec![
        vec!["provider".into(), config.provider.clone(), source_for("provider")],
        vec!["language".into(), config.language.clone(), source_for("language")],
        vec!["pr_target".into(), config.pr_target.clone(), source_for("pr_target")],
        vec!["platform".into(), config.platform.clone(), source_for("platform")],
        vec!["vcs".into(), config.vcs.clone(), source_for("vcs")],
        vec!["model".into(), model.to_string(), source_for("provider")],
    ];

    print!("{}", format.render_rows(&headers, &rows, is_tty));
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::config::ProviderReadiness;

    #[test]
    fn readiness_hint_labels() {
        assert_eq!(readiness_hint(ProviderReadiness::Ready), "ready");
        assert_eq!(readiness_hint(ProviderReadiness::NeedsSetup), "needs setup");
        assert_eq!(readiness_hint(ProviderReadiness::NotInstalled), "not installed");
    }

    #[test]
    fn next_step_hint_ready_is_none() {
        assert!(next_step_hint("anthropic", ProviderReadiness::Ready).is_none());
    }

    #[test]
    fn next_step_hint_codex_points_at_login() {
        let h = next_step_hint("codex", ProviderReadiness::NeedsSetup).unwrap();
        assert!(h.contains("codex login"));
    }

    #[test]
    fn next_step_hint_api_points_at_key() {
        let h = next_step_hint("anthropic", ProviderReadiness::NeedsSetup).unwrap();
        assert!(h.contains("anthropic"));
    }

    #[test]
    fn next_step_hint_cli_points_at_path() {
        let h = next_step_hint("claude", ProviderReadiness::NotInstalled).unwrap();
        assert!(h.contains("PATH"));
    }
}
