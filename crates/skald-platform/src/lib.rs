pub mod github;

use serde::Deserialize;

pub use github::GitHubAdapter;

#[derive(Debug, Clone, Deserialize)]
pub struct PrInfo {
    pub number: u64,
    pub url: String,
    pub state: String,
    pub title: String,
    #[serde(rename = "headRefName")]
    pub head_branch: String,
    #[serde(rename = "baseRefName")]
    pub base_branch: String,
}

#[derive(Debug, Clone)]
pub struct CreatePrRequest {
    pub title: String,
    pub body: String,
    pub base: String,
    pub draft: bool,
    pub push: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error(
        "{cli} CLI not found. Install it from: {install_url}"
    )]
    CliNotFound { cli: String, install_url: String },

    #[error(
        "Not authenticated with {cli}. Run: {auth_command}"
    )]
    NotAuthenticated { cli: String, auth_command: String },

    #[error("API error: {detail}")]
    ApiError { detail: String },

    #[error("{0}")]
    Other(String),
}

pub trait PlatformAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn pr_exists(&self, branch: &str) -> Result<Option<PrInfo>, PlatformError>;
    fn create_pr(&self, request: &CreatePrRequest) -> Result<PrInfo, PlatformError>;
}

pub fn detect_platform(remote_url: &str) -> Option<Box<dyn PlatformAdapter>> {
    if let Some(adapter) = GitHubAdapter::detect(remote_url) {
        return Some(Box::new(adapter));
    }
    None
}
