use thiserror::Error;

#[derive(Debug, Error)]
pub enum SkaldError {
    #[error("Configuration file not found at {path}. Run `sk config init` to create one.")]
    ConfigNotFound { path: String },

    #[error("Failed to parse config at {path}, line {line}: {detail}")]
    ConfigParse { path: String, line: usize, detail: String },

    #[error("Provider '{provider}' not configured. Run `sk config init` to set up.")]
    ProviderNotConfigured { provider: String },

    #[error("Provider '{provider}' failed: {detail}")]
    ProviderError { provider: String, detail: String },

    #[error("Not in a git repository. Run from inside a repo, or run `git init`.")]
    NotInRepo,

    #[error("No staged changes. Stage files with `git add` or use `sk commit -a`.")]
    NoStagedChanges,

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{message}")]
    Other { message: String },
}

pub type Result<T> = std::result::Result<T, SkaldError>;

impl SkaldError {
    pub fn suggestion(&self) -> Option<&str> {
        match self {
            Self::ConfigNotFound { .. } => Some("Run `sk config init` to create a default config."),
            Self::ProviderNotConfigured { .. } => {
                Some("Run `sk config init` to set up a provider.")
            }
            Self::NotInRepo => Some("Navigate to a git repository or run `git init`."),
            Self::NoStagedChanges => {
                Some("Stage files with `git add <files>` or use `sk commit -a` to auto-stage.")
            }
            _ => None,
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Io(_) | Self::Other { .. } => 1,
            Self::ConfigNotFound { .. }
            | Self::ConfigParse { .. }
            | Self::ProviderNotConfigured { .. }
            | Self::ProviderError { .. }
            | Self::NotInRepo
            | Self::NoStagedChanges => 1,
        }
    }
}
