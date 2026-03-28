mod cli;
mod ui;

use clap::Parser;
use cli::{Cli, Command, ConfigAction};
use std::process;

fn main() {
    let raw_args: Vec<String> = std::env::args().collect();

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
    let is_tty = atty::is(atty::Stream::Stdout);

    let code = match cli.command {
        Command::Completions { shell } => {
            cli::completions::run(shell);
            0
        }
        Command::Commit { show_prompt } => {
            if show_prompt {
                let ctx = skald_core::prompts::mock_prompt_context();
                match skald_core::prompts::resolve_template("commit-title", None, None) {
                    Ok(template) => match skald_core::prompts::render_prompt(&template, &ctx) {
                        Ok(rendered) => {
                            print!("{rendered}");
                            0
                        }
                        Err(e) => {
                            cliclack::log::error(e.to_string()).ok();
                            1
                        }
                    },
                    Err(e) => {
                        cliclack::log::error(e.to_string()).ok();
                        1
                    }
                }
            } else {
                cliclack::log::warning("Not yet implemented — coming in M4.").ok();
                0
            }
        }
        Command::Pr { show_prompt } => {
            if show_prompt {
                let ctx = skald_core::prompts::mock_prompt_context();
                match skald_core::prompts::resolve_template("pr-title", None, None) {
                    Ok(template) => match skald_core::prompts::render_prompt(&template, &ctx) {
                        Ok(rendered) => {
                            print!("{rendered}");
                            0
                        }
                        Err(e) => {
                            cliclack::log::error(e.to_string()).ok();
                            1
                        }
                    },
                    Err(e) => {
                        cliclack::log::error(e.to_string()).ok();
                        1
                    }
                }
            } else {
                cliclack::log::warning("Not yet implemented — coming in M8.").ok();
                0
            }
        }
        Command::Config { action } => {
            let action = action.unwrap_or(ConfigAction::Show);
            match action {
                ConfigAction::Init => cli::config::run_init(),
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
        Command::Aliases { source } => match config_result {
            Ok(ref cfg) => cli::aliases::run_aliases(cfg, fmt, is_tty, source),
            Err(ref e) => {
                cliclack::log::error(format!("Failed to load config: {e}")).ok();
                1
            }
        },
        Command::Doctor { fix } => cli::doctor::run_doctor(fix, fmt, is_tty),
    };

    process::exit(code);
}
