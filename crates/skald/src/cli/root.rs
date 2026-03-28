use clap::Parser;
use skald_core::output::OutputFormat;

#[derive(Parser, Debug)]
#[command(
    name = "sk",
    about = "AI-powered git workflow CLI",
    long_about = "Skald — generate commit messages, PR titles, and PR descriptions with AI.",
    version,
    propagate_version = true,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Increase verbosity (-v info, -vv debug, -vvv trace)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress all output except errors and final results
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Disable color output
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Output format
    #[arg(long, value_enum, global = true)]
    pub format: Option<OutputFormat>,
}

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    /// Generate commit messages and commit
    Commit,
    /// Generate PR title and description
    Pr,
    /// View and manage configuration
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    /// List active aliases and their sources
    Aliases {
        /// Show which config file each alias comes from
        #[arg(long)]
        source: bool,
    },
    /// Validate environment, config, and provider connectivity
    Doctor,
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum ConfigAction {
    /// Create a default global config file
    Init,
    /// Display the resolved configuration
    Show,
}

impl Cli {
    pub fn effective_format(&self) -> OutputFormat {
        if let Some(fmt) = self.format {
            return fmt;
        }
        if atty::is(atty::Stream::Stdout) { OutputFormat::Table } else { OutputFormat::Plain }
    }

    pub fn should_use_color(&self) -> bool {
        if self.no_color {
            return false;
        }
        if std::env::var_os("NO_COLOR").is_some() {
            return false;
        }
        atty::is(atty::Stream::Stdout)
    }
}
