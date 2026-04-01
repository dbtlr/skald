use std::io::IsTerminal;

use crate::engine::output::OutputFormat;
use clap::Parser;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum ColorWhen {
    Auto,
    Always,
    Never,
}

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

    /// When to use color output
    #[arg(long, value_enum, default_value = "auto", global = true)]
    pub color: ColorWhen,

    /// Output format
    #[arg(long, value_enum, global = true)]
    pub format: Option<OutputFormat>,

    /// AI provider to use
    #[arg(long, global = true)]
    pub provider: Option<String>,

    /// Model to use for AI generation
    #[arg(short = 'm', long, global = true)]
    pub model: Option<String>,

    /// Run as if started in <path>
    #[arg(short = 'C', long = "cwd", global = true, value_name = "PATH")]
    pub directory: Option<std::path::PathBuf>,
}

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    /// Generate commit messages and commit
    Commit {
        /// Generate one message and commit immediately (implies -n 1)
        #[arg(short = 'y', long = "yes")]
        yes: bool,
        /// Number of suggestions to generate
        #[arg(short = 'n', long = "num", default_value = "3")]
        count: usize,
        /// Stage tracked modified files before committing (git add -u)
        #[arg(short = 'a', long = "all")]
        all: bool,
        /// Also stage untracked files (implies -a)
        #[arg(long)]
        include_untracked: bool,
        /// Amend the previous commit
        #[arg(long)]
        amend: bool,
        /// Provide context about the changes
        #[arg(short = 'c', long)]
        context: Option<String>,
        /// Read context from a file
        #[arg(long)]
        context_file: Option<std::path::PathBuf>,
        /// Print what would be committed without executing
        #[arg(long)]
        dry_run: bool,
        /// Generate commit body (multi-line description)
        #[arg(long)]
        body: bool,
    },
    /// Generate PR title and description
    Pr {
        /// Generate title + description and create PR immediately (implies -n 1)
        #[arg(short = 'y', long = "yes")]
        yes: bool,
        /// Print full PR payload without creating
        #[arg(long)]
        dry_run: bool,
        /// Create as draft PR
        #[arg(short = 'd', long)]
        draft: bool,
        /// Push current branch to remote before creating PR
        #[arg(long)]
        push: bool,
        /// Update existing PR title and description
        #[arg(long)]
        update: bool,
        /// Target branch
        #[arg(short = 'b', long)]
        base: Option<String>,
        /// Number of title suggestions
        #[arg(short = 'n', long = "num", default_value = "3")]
        count: usize,
        /// Provide context about the PR
        #[arg(short = 'c', long)]
        context: Option<String>,
        /// Read context from a file
        #[arg(long)]
        context_file: Option<std::path::PathBuf>,
    },
    /// Generate merge request title and description (alias for pr)
    Mr {
        /// Generate title + description and create MR immediately (implies -n 1)
        #[arg(short = 'y', long = "yes")]
        yes: bool,
        /// Print full MR payload without creating
        #[arg(long)]
        dry_run: bool,
        /// Create as draft MR
        #[arg(short = 'd', long)]
        draft: bool,
        /// Push current branch to remote before creating MR
        #[arg(long)]
        push: bool,
        /// Update existing MR title and description
        #[arg(long)]
        update: bool,
        /// Target branch
        #[arg(short = 'b', long)]
        base: Option<String>,
        /// Number of title suggestions
        #[arg(short = 'n', long = "num", default_value = "3")]
        count: usize,
        /// Provide context about the MR
        #[arg(short = 'c', long)]
        context: Option<String>,
        /// Read context from a file
        #[arg(long)]
        context_file: Option<std::path::PathBuf>,
    },
    /// View and manage configuration
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    /// Manage aliases
    #[command(alias = "aliases", arg_required_else_help = true)]
    Alias {
        #[command(subcommand)]
        action: AliasAction,
    },
    /// Validate environment, config, and provider connectivity
    Doctor {
        /// Auto-fix all fixable issues
        #[arg(long)]
        fix: bool,
        /// Skip network connectivity checks
        #[arg(long)]
        offline: bool,
    },
    /// Check for and install updates
    Upgrade {
        /// Show what would happen without downloading
        #[arg(long)]
        dry_run: bool,
    },
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Output integration config snippets for external tools (experimental)
    #[cfg(feature = "integrations")]
    Integrations {
        #[command(subcommand)]
        target: Option<IntegrationTarget>,
    },
}

#[cfg(feature = "integrations")]
#[derive(clap::Subcommand, Debug)]
pub enum IntegrationTarget {
    /// Worktrunk commit message config
    Worktrunk,
    /// Lazygit custom command config
    Lazygit,
    /// Vim-fugitive keybinding config
    Fugitive,
    /// Git prepare-commit-msg hook
    Hook {
        /// Install the hook to .git/hooks/
        #[arg(long)]
        install: bool,
        /// Overwrite existing hook file
        #[arg(long)]
        force: bool,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum ConfigAction {
    /// Create a default global config file
    Init {
        /// Initialize with a specific provider
        #[arg(long)]
        provider: Option<String>,
        /// Initialize with a specific model
        #[arg(long)]
        model: Option<String>,
    },
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

#[derive(clap::Subcommand, Debug)]
pub enum AliasAction {
    /// List all active aliases
    List,
    /// Add a new alias
    Add {
        /// Alias name
        name: String,
        /// Command expansion (e.g. "commit -n 5")
        expansion: String,
        /// Write to project config (.skaldrc.yaml) instead of global
        #[arg(long)]
        project: bool,
        /// Overwrite an existing alias
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// Remove an alias
    Remove {
        /// Alias name to remove
        name: String,
        /// Remove from project config (.skaldrc.yaml) instead of global
        #[arg(long)]
        project: bool,
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
        match self.color {
            ColorWhen::Always => true,
            ColorWhen::Never => false,
            ColorWhen::Auto => {
                if std::env::var_os("NO_COLOR").is_some() {
                    return false;
                }
                std::io::stdout().is_terminal()
            }
        }
    }
}
