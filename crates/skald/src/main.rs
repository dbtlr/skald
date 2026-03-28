mod cli;
mod ui;

use clap::Parser;
use cli::{Cli, Command};
use std::process;

fn main() {
    let cli = Cli::parse();

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

    let code = match cli.command {
        Command::Completions { shell } => {
            cli::completions::run(shell);
            0
        }
        Command::Commit => {
            cliclack::log::warning("Not yet implemented — coming in M1.").ok();
            0
        }
        Command::Pr => {
            cliclack::log::warning("Not yet implemented — coming in M2.").ok();
            0
        }
        Command::Config => {
            let fmt = cli.effective_format();
            let is_tty = atty::is(atty::Stream::Stdout);
            let headers = vec!["Key", "Value", "Source"];
            let rows = vec![
                vec!["provider".into(), "(not configured)".into(), "default".into()],
                vec!["model".into(), "(not configured)".into(), "default".into()],
            ];
            print!("{}", fmt.render_rows(&headers, &rows, is_tty));
            0
        }
        Command::Aliases => {
            cliclack::log::warning("Not yet implemented — coming in M1.").ok();
            0
        }
        Command::Doctor => {
            cliclack::log::warning("Not yet implemented — coming in M3.").ok();
            0
        }
    };

    process::exit(code);
}
