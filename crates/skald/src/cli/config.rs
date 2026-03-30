use skald_core::config::{ResolvedConfig, global_config_path};
use skald_core::output::OutputFormat;
use skald_providers::config::{
    available_provider_names, get_provider_config, is_provider_available,
};
use skald_providers::models::{get_model_list, get_opencode_models, models_for_provider};

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

pub fn run_init(provider_arg: Option<&str>, model_arg: Option<&str>, is_tty: bool) -> i32 {
    // With --provider flag: validate, check availability, write directly
    if let Some(provider) = provider_arg {
        if get_provider_config(provider).is_none() {
            let known = available_provider_names().join(", ");
            cliclack::log::error(format!(
                "Unknown provider '{provider}'. Known providers: {known}"
            ))
            .ok();
            return 1;
        }
        if !is_provider_available(provider) {
            cliclack::log::warning(format!(
                "Provider '{provider}' binary not found in PATH. Config will be written anyway."
            ))
            .ok();
        }
        return write_config(provider, model_arg);
    }

    let all_names = available_provider_names();
    let found: Vec<&str> =
        all_names.iter().copied().filter(|name| is_provider_available(name)).collect();

    // Non-interactive: show detection results and suggest command
    if !is_tty {
        eprintln!("error: No provider specified. Skald needs an AI provider to work.");
        eprintln!();
        eprintln!("Available providers detected:");
        for name in &all_names {
            let marker = if found.contains(name) { "✓" } else { "✗" };
            let status = if found.contains(name) { "found" } else { "not found" };
            eprintln!("  {marker} {name:<12} ({status})");
        }
        eprintln!();
        if let Some(first) = found.first() {
            eprintln!("Run: sk config init --provider {first}");
        } else {
            eprintln!("No providers found. Install one to get started.");
            eprintln!("  claude: https://claude.ai/download");
            eprintln!("  codex:  https://github.com/openai/codex");
            eprintln!("  gemini: https://github.com/google-gemini/gemini-cli");
        }
        return 1;
    }

    // Interactive: no providers available
    if found.is_empty() {
        cliclack::log::error(
            "No AI providers found in PATH. Install one to get started:\n  claude: https://claude.ai/download\n  codex:  https://github.com/openai/codex\n  gemini: https://github.com/google-gemini/gemini-cli"
        ).ok();
        return 1;
    }

    // Interactive: select provider
    let provider_options: Vec<(&str, &str, &str)> =
        found.iter().map(|&name| (name, name, "")).collect();

    let selected_provider =
        match cliclack::select("Select an AI provider").items(&provider_options).interact() {
            Ok(p) => p,
            Err(_) => return 1,
        };

    // Interactive: prompt for model using picker
    let model = resolve_init_model(model_arg, selected_provider, is_tty);

    write_config(selected_provider, model.as_deref())
}

pub fn run_eject(project: bool, name: Option<&str>) -> i32 {
    let target_dir = if project {
        std::env::current_dir().unwrap_or_default().join(".skald").join("prompts")
    } else {
        skald_core::config::config_dir().join("prompts")
    };

    let names: Option<Vec<&str>> = name.map(|n| vec![n]);
    let names_ref = names.as_deref();

    match skald_core::prompts::eject_prompts(&target_dir, names_ref) {
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
