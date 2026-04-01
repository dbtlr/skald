use crate::engine::config::{self, ResolvedConfig};
use crate::engine::output::OutputFormat;

pub fn run_list(config: &ResolvedConfig, format: OutputFormat, is_tty: bool) -> i32 {
    if config.aliases.is_empty() {
        cliclack::log::info("No aliases configured.").ok();
        return 0;
    }

    let mut sorted: Vec<(&String, &String)> = config.aliases.iter().collect();
    sorted.sort_by_key(|(name, _)| name.as_str());

    let headers = vec!["Alias", "Expansion", "Source"];
    let rows: Vec<Vec<String>> = sorted
        .iter()
        .map(|(name, expansion)| {
            let source = config
                .sources
                .get(format!("alias.{name}").as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "config".to_string());
            vec![(*name).clone(), (*expansion).clone(), source]
        })
        .collect();
    print!("{}", format.render_rows(&headers, &rows, is_tty));

    0
}

pub fn run_add(name: &str, expansion: &str, project: bool, force: bool) -> i32 {
    let path = resolve_config_path(project);
    match config::add_alias(name, expansion, &path, force) {
        Ok(()) => {
            let scope = if project { "project" } else { "global" };
            cliclack::log::success(format!("Added alias '{name}' → \"{expansion}\" ({scope})"))
                .ok();
            0
        }
        Err(e) => {
            cliclack::log::error(format!("{e}")).ok();
            if let Some(hint) = e.suggestion() {
                cliclack::log::info(hint).ok();
            }
            1
        }
    }
}

pub fn run_remove(name: &str, project: bool) -> i32 {
    let path = resolve_config_path(project);
    let scope = if project { "project" } else { "global" };
    match config::remove_alias(name, &path, scope) {
        Ok(()) => {
            cliclack::log::success(format!("Removed alias '{name}' from {scope} config")).ok();
            0
        }
        Err(e) => {
            cliclack::log::error(format!("{e}")).ok();
            if let Some(hint) = e.suggestion() {
                cliclack::log::info(hint).ok();
            }
            1
        }
    }
}

fn resolve_config_path(project: bool) -> std::path::PathBuf {
    if project {
        let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
        config::discover_project_config(&cwd).unwrap_or_else(|| cwd.join(".skaldrc.yaml"))
    } else {
        config::global_config_path()
    }
}
