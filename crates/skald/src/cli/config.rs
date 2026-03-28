use skald_core::config::{ResolvedConfig, global_config_path};
use skald_core::output::OutputFormat;

const DEFAULT_CONFIG_TEMPLATE: &str = r#"# Skald configuration
# See: https://github.com/dbtlr/skald/docs/configuration.md

# AI provider (default: claude-cli)
# provider: claude-cli

# Language for generated messages (default: English)
# language: English

# Default PR target branch (default: main)
# pr_target: main

# Provider-specific settings
# providers:
#   claude-cli:
#     model: claude-sonnet-4-20250514
#   anthropic-api:
#     api_key: $ANTHROPIC_API_KEY
#     model: claude-haiku-4-5

# Aliases — composable flag shortcuts
# aliases:
#   ci: "commit -n 5"
#   ca: "commit --auto -A"
#   fix: "commit --auto -a --context 'bug fix'"
"#;

pub fn run_init() -> i32 {
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

    if let Err(e) = std::fs::write(&path, DEFAULT_CONFIG_TEMPLATE) {
        cliclack::log::error(format!("Failed to write config: {e}")).ok();
        return 1;
    }

    cliclack::log::success(format!("Config created at {}", path.display())).ok();
    0
}

pub fn run_eject(project: bool, name: Option<&str>) -> i32 {
    let target_dir = if project {
        std::env::current_dir().unwrap_or_default().join(".tool").join("prompts")
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
