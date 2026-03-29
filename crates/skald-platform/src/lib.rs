pub mod github;
pub mod gitlab;

use serde::Deserialize;

pub use github::GitHubAdapter;
pub use gitlab::GitLabAdapter;

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
    #[error("{cli} CLI not found. Install it from: {install_url}")]
    CliNotFound { cli: String, install_url: String },

    #[error("Not authenticated with {cli}. Run: {auth_command}")]
    NotAuthenticated { cli: String, auth_command: String },

    #[error("API error: {detail}")]
    ApiError { detail: String },

    #[error("{0}")]
    Other(String),
}

pub trait PlatformAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn pr_label(&self) -> &str;
    fn pr_prefix(&self) -> &str;
    fn pr_exists(&self, branch: &str) -> Result<Option<PrInfo>, PlatformError>;
    fn create_pr(&self, request: &CreatePrRequest) -> Result<PrInfo, PlatformError>;
    fn update_pr(&self, branch: &str, title: &str, body: &str) -> Result<PrInfo, PlatformError>;
}

/// Detect the platform adapter.
///
/// Resolution order:
/// 1. `config_platform` override: `"github"` → GitHubAdapter, `"gitlab"` → GitLabAdapter,
///    `"auto"` or unknown → fall through to URL matching.
/// 2. URL matching: `github.com` → GitHub, `gitlab.com`/`gitlab.` → GitLab, else None.
pub fn detect_platform(
    remote_url: &str,
    config_platform: Option<&str>,
) -> Option<Box<dyn PlatformAdapter>> {
    // Config override
    match config_platform {
        Some("github") => return Some(Box::new(GitHubAdapter)),
        Some("gitlab") => return Some(Box::new(GitLabAdapter)),
        Some("auto") | None => {}
        Some(_) => {}
    }

    // URL-based detection
    if remote_url.contains("github.com") {
        return Some(Box::new(GitHubAdapter));
    }
    if remote_url.contains("gitlab.com") || remote_url.contains("gitlab.") {
        return Some(Box::new(GitLabAdapter));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_override_forces_github_regardless_of_url() {
        let result = detect_platform("https://gitlab.com/user/repo.git", Some("github"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().name(), "github");
    }

    #[test]
    fn config_override_forces_gitlab_regardless_of_url() {
        let result = detect_platform("https://github.com/user/repo.git", Some("gitlab"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().name(), "gitlab");
    }

    #[test]
    fn config_auto_falls_through_to_url_matching() {
        let result = detect_platform("https://github.com/user/repo.git", Some("auto"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().name(), "github");
    }

    #[test]
    fn url_matching_github_com() {
        let result = detect_platform("https://github.com/user/repo.git", None);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name(), "github");
    }

    #[test]
    fn url_matching_gitlab_com() {
        let result = detect_platform("https://gitlab.com/user/repo.git", None);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name(), "gitlab");
    }

    #[test]
    fn url_matching_self_hosted_gitlab() {
        let result = detect_platform("https://gitlab.company.com/user/repo.git", None);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name(), "gitlab");
    }

    #[test]
    fn url_matching_unknown_returns_none() {
        let result = detect_platform("https://bitbucket.org/user/repo.git", None);
        assert!(result.is_none());
    }
}
