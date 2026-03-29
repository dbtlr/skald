use std::process::Command;

use crate::{CreatePrRequest, PlatformAdapter, PlatformError, PrInfo};

pub struct GitLabAdapter;

impl GitLabAdapter {
    pub fn detect(remote_url: &str) -> Option<Self> {
        if remote_url.contains("gitlab.com") || remote_url.contains("gitlab.") {
            Some(GitLabAdapter)
        } else {
            None
        }
    }

    pub fn is_available() -> bool {
        Command::new("glab").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
    }
}

impl PlatformAdapter for GitLabAdapter {
    fn name(&self) -> &str {
        "gitlab"
    }

    fn pr_label(&self) -> &str {
        "MR"
    }

    fn pr_prefix(&self) -> &str {
        "!"
    }

    fn pr_exists(&self, _branch: &str) -> Result<Option<PrInfo>, PlatformError> {
        todo!("GitLab MR support not yet implemented")
    }

    fn create_pr(&self, _request: &CreatePrRequest) -> Result<PrInfo, PlatformError> {
        todo!("GitLab MR support not yet implemented")
    }

    fn update_pr(&self, _branch: &str, _title: &str, _body: &str) -> Result<PrInfo, PlatformError> {
        todo!("GitLab MR support not yet implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_gitlab_com_https() {
        assert!(GitLabAdapter::detect("https://gitlab.com/user/repo.git").is_some());
    }

    #[test]
    fn detect_gitlab_com_ssh() {
        assert!(GitLabAdapter::detect("git@gitlab.com:user/repo.git").is_some());
    }

    #[test]
    fn detect_self_hosted_gitlab() {
        assert!(GitLabAdapter::detect("https://gitlab.company.com/user/repo.git").is_some());
    }

    #[test]
    fn detect_non_gitlab_returns_none() {
        assert!(GitLabAdapter::detect("https://github.com/user/repo.git").is_none());
        assert!(GitLabAdapter::detect("https://bitbucket.org/user/repo.git").is_none());
    }

    #[test]
    fn terminology_label_is_mr() {
        assert_eq!(GitLabAdapter.pr_label(), "MR");
    }

    #[test]
    fn terminology_prefix_is_exclamation() {
        assert_eq!(GitLabAdapter.pr_prefix(), "!");
    }
}
