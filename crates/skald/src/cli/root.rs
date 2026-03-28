use std::io::IsTerminal;

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
    Commit {
        /// Render the prompt and print to stdout without calling AI
        #[arg(long)]
        show_prompt: bool,
        /// Generate one message and commit immediately
        #[arg(long)]
        auto: bool,
        /// Print messages to stdout without committing
        #[arg(long)]
        message_only: bool,
        /// Number of suggestions to generate
        #[arg(short = 'n', long = "num", default_value = "3")]
        count: usize,
        /// Stage tracked modified files before committing (git add -u)
        #[arg(short = 'a')]
        stage_tracked: bool,
        /// Stage all files including untracked (git add -A)
        #[arg(short = 'A', long = "all")]
        stage_all: bool,
        /// Amend the previous commit
        #[arg(long)]
        amend: bool,
        /// Provide context about the changes
        #[arg(long)]
        context: Option<String>,
        /// Read context from a file
        #[arg(long)]
        context_file: Option<std::path::PathBuf>,
        /// Print what would be committed without executing
        #[arg(long)]
        dry_run: bool,
    },
    /// Generate PR title and description
    Pr {
        /// Render the prompt and print to stdout without calling AI
        #[arg(long)]
        show_prompt: bool,
    },
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
    Doctor {
        /// Auto-fix all fixable issues
        #[arg(long)]
        fix: bool,
    },
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
    /// Eject prompt templates for customization
    Eject {
        /// Eject to project directory (.skald/prompts/) instead of global
        #[arg(long)]
        project: bool,
        /// Specific template name to eject (ejects all if omitted)
        name: Option<String>,
    },
}

impl Cli {
    pub fn effective_format(&self) -> OutputFormat {
        if let Some(fmt) = self.format {
            return fmt;
        }
        if std::io::stdout().is_terminal() { OutputFormat::Table } else { OutputFormat::Plain }
    }

    pub fn should_use_color(&self) -> bool {
        if self.no_color {
            return false;
        }
        if std::env::var_os("NO_COLOR").is_some() {
            return false;
        }
        std::io::stdout().is_terminal()
    }
}
