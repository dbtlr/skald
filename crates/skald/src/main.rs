mod cli;
mod ui;

use std::io::IsTerminal;
use std::process;

use clap::Parser;
use cli::{Cli, Command, ConfigAction};

fn main() {
    let raw_args: Vec<String> = std::env::args().collect();

    // Pre-scan for -C <path> and chdir before config loading.
    // Config discovery walks up from cwd, so this must happen first.
    if let Some(dir) = prescan_directory(&raw_args)
        && let Err(e) = std::env::set_current_dir(&dir)
    {
        eprintln!("error: cannot change to directory '{}': {e}", dir.display());
        process::exit(1);
    }

    // Load config early (before clap). Non-fatal — store Result.
    let config_result = skald_core::config::load_config();

    // Expand aliases if config loaded and has aliases
    let effective_args = if let Ok(ref cfg) = config_result {
        if let Some(expanded) = skald_core::config::expand_alias(&raw_args[1..], &cfg.aliases) {
            let mut full = vec![raw_args[0].clone()];
            full.extend(expanded);
            full
        } else {
            raw_args.clone()
        }
    } else {
        raw_args.clone()
    };

    let cli = Cli::parse_from(&effective_args);

    // Initialize color state
    let use_color = cli.should_use_color();
    ui::color::init(!use_color);

    if use_color {
        ui::theme::SkaldTheme::apply();
    }

    // Initialize logging (holds guard for file appender lifetime)
    let _log_guard = skald_core::logging::init(cli.verbose, cli.quiet);

    tracing::debug!(
        verbose = cli.verbose,
        quiet = cli.quiet,
        no_color = cli.no_color,
        format = ?cli.effective_format(),
        "skald starting"
    );

    let fmt = cli.effective_format();
    let is_tty = std::io::stdout().is_terminal();

    // Resolve provider name: --provider flag → config → "claude"
    let provider_name = cli.provider.clone().unwrap_or_else(|| match config_result {
        Ok(ref cfg) => cfg.provider.clone(),
        Err(_) => "claude".to_string(),
    });

    // Resolve model: --model flag → config providers.<name>.model → None
    let model = cli.model.clone().or_else(|| {
        config_result
            .as_ref()
            .ok()
            .and_then(|cfg| cfg.providers.get(&provider_name).and_then(|p| p.model.clone()))
    });

    let code = match cli.command {
        Command::Completions { shell } => {
            cli::completions::run(shell);
            0
        }
        Command::Commit {
            show_prompt,
            auto,
            message_only,
            count,
            stage_tracked,
            stage_all,
            amend,
            context,
            context_file,
            dry_run,
            extended,
        } => {
            let config = match config_result {
                Ok(ref cfg) => cfg,
                Err(ref e) => {
                    cliclack::log::error(format!("Failed to load config: {e}")).ok();
                    process::exit(1);
                }
            };
            cli::commit::run_commit(
                cli::commit::CommitOptions {
                    show_prompt,
                    auto,
                    message_only,
                    count,
                    stage_tracked,
                    stage_all,
                    amend,
                    context,
                    context_file,
                    dry_run,
                    extended,
                    format: fmt,
                    is_tty,
                    provider_name: provider_name.clone(),
                    model: model.clone(),
                },
                config,
            )
        }
        Command::Pr {
            show_prompt,
            auto,
            title_only,
            dry_run,
            draft,
            push,
            update,
            base,
            count,
            context,
        } => {
            let config = match config_result {
                Ok(ref cfg) => cfg,
                Err(ref e) => {
                    cliclack::log::error(format!("Failed to load config: {e}")).ok();
                    process::exit(1);
                }
            };
            cli::pr::run_pr(
                cli::pr::PrOptions {
                    show_prompt,
                    auto,
                    title_only,
                    dry_run,
                    draft,
                    push,
                    update,
                    base,
                    count,
                    context,
                    format: fmt,
                    is_tty,
                    provider_name: provider_name.clone(),
                    model: model.clone(),
                },
                config,
            )
        }
        Command::Mr {
            show_prompt,
            auto,
            title_only,
            dry_run,
            draft,
            push,
            update,
            base,
            count,
            context,
        } => {
            let config = match config_result {
                Ok(ref cfg) => cfg,
                Err(ref e) => {
                    cliclack::log::error(format!("Failed to load config: {e}")).ok();
                    process::exit(1);
                }
            };
            cli::pr::run_pr(
                cli::pr::PrOptions {
                    show_prompt,
                    auto,
                    title_only,
                    dry_run,
                    draft,
                    push,
                    update,
                    base,
                    count,
                    context,
                    format: fmt,
                    is_tty,
                    provider_name: provider_name.clone(),
                    model: model.clone(),
                },
                config,
            )
        }
        Command::Config { action } => {
            let action = action.unwrap_or(ConfigAction::Show);
            match action {
                ConfigAction::Init { provider, model } => {
                    cli::config::run_init(provider.as_deref(), model.as_deref(), is_tty)
                }
                ConfigAction::Show => match config_result {
                    Ok(ref cfg) => cli::config::run_show(cfg, fmt, is_tty),
                    Err(ref e) => {
                        cliclack::log::error(format!("Failed to load config: {e}")).ok();
                        1
                    }
                },
                ConfigAction::Eject { project, name } => {
                    cli::config::run_eject(project, name.as_deref())
                }
            }
        }
        Command::Alias { source } => match config_result {
            Ok(ref cfg) => cli::aliases::run_aliases(cfg, fmt, is_tty, source),
            Err(ref e) => {
                cliclack::log::error(format!("Failed to load config: {e}")).ok();
                1
            }
        },
        Command::Doctor { fix, full } => cli::doctor::run_doctor(fix, full, fmt, is_tty),
        Command::Upgrade { dry_run } => cli::upgrade::run_upgrade(dry_run),
    };

    process::exit(code);
}

/// Pre-scan raw args for `-C <path>` before clap parsing.
/// Returns the path if found. This runs before config loading
/// so that project config discovery uses the correct cwd.
fn prescan_directory(args: &[String]) -> Option<std::path::PathBuf> {
    let mut iter = args.iter().skip(1); // skip binary name
    while let Some(arg) = iter.next() {
        if arg == "-C" {
            return iter.next().map(std::path::PathBuf::from);
        }
        if let Some(path) = arg.strip_prefix("-C") {
            return Some(std::path::PathBuf::from(path));
        }
    }
    None
}
