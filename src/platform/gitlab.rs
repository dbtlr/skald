use std::process::Command;

use tracing::debug;

use crate::platform::{CreatePrRequest, PlatformAdapter, PlatformError, PrInfo};

pub struct GitLabAdapter;

#[derive(Debug, Clone, serde::Deserialize)]
struct GitLabMrInfo {
    iid: u64,
    web_url: String,
    state: String,
    title: String,
    source_branch: String,
    target_branch: String,
}

impl From<GitLabMrInfo> for PrInfo {
    fn from(mr: GitLabMrInfo) -> Self {
        PrInfo {
            number: mr.iid,
            url: mr.web_url,
            state: mr.state,
            title: mr.title,
            head_branch: mr.source_branch,
            base_branch: mr.target_branch,
        }
    }
}

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

    fn check_available(&self) -> Result<(), PlatformError> {
        if !Self::is_available() {
            return Err(PlatformError::CliNotFound {
                cli: "glab".to_string(),
                install_url: "https://gitlab.com/gitlab-org/cli".to_string(),
            });
        }
        Ok(())
    }

    fn check_auth_error(stderr: &str) -> Option<PlatformError> {
        if stderr.contains("not logged in")
            || stderr.contains("authentication")
            || stderr.contains("401")
            || stderr.contains("GITLAB_TOKEN")
        {
            Some(PlatformError::NotAuthenticated {
                cli: "glab".to_string(),
                auth_command: "glab auth login".to_string(),
            })
        } else {
            None
        }
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

    fn pr_exists(&self, branch: &str) -> Result<Option<PrInfo>, PlatformError> {
        self.check_available()?;

        debug!("Checking for existing MR for branch: {}", branch);

        let output = Command::new("glab")
            .args(["mr", "list", "--source-branch", branch, "-F", "json"])
            .output()
            .map_err(|e| PlatformError::Other(format!("Failed to run glab: {e}")))?;

        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            if let Some(auth_err) = Self::check_auth_error(&stderr) {
                return Err(auth_err);
            }
            return Err(PlatformError::ApiError { detail: stderr.trim().to_string() });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        debug!("glab mr list output: {}", stdout.trim());

        let mrs: Vec<GitLabMrInfo> = serde_json::from_str(&stdout)
            .map_err(|e| PlatformError::Other(format!("Failed to parse glab output: {e}")))?;

        Ok(mrs.into_iter().next().map(PrInfo::from))
    }

    fn create_pr(&self, request: &CreatePrRequest) -> Result<PrInfo, PlatformError> {
        self.check_available()?;

        if request.push {
            debug!("Pushing branch to origin before creating MR");
            let push_output =
                Command::new("git")
                    .args(["push", "-u", "origin", "HEAD"])
                    .output()
                    .map_err(|e| PlatformError::Other(format!("Failed to run git push: {e}")))?;

            if !push_output.status.success() {
                let stderr = String::from_utf8_lossy(&push_output.stderr);
                return Err(PlatformError::Other(format!("git push failed: {}", stderr.trim())));
            }
        }

        debug!("Creating MR: {}", request.title);

        let mut args = vec![
            "mr",
            "create",
            "--title",
            &request.title,
            "--description",
            &request.body,
            "--target-branch",
            &request.base,
            "--yes",
        ];

        if request.draft {
            args.push("--draft");
        }

        let output = Command::new("glab")
            .args(&args)
            .output()
            .map_err(|e| PlatformError::Other(format!("Failed to run glab: {e}")))?;

        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            if let Some(auth_err) = Self::check_auth_error(&stderr) {
                return Err(auth_err);
            }
            return Err(PlatformError::ApiError { detail: stderr.trim().to_string() });
        }

        // After creation, fetch structured MR info via `glab mr view`
        let view_output = Command::new("glab").args(["mr", "view", "--output", "json"]).output();

        match view_output {
            Ok(view) if view.status.success() => {
                let view_stdout = String::from_utf8_lossy(&view.stdout);
                debug!("glab mr view output: {}", view_stdout.trim());
                match serde_json::from_str::<GitLabMrInfo>(&view_stdout) {
                    Ok(mr) => Ok(PrInfo::from(mr)),
                    Err(e) => {
                        debug!(
                            "Failed to parse glab mr view output: {e}, falling back to minimal PrInfo"
                        );
                        fallback_mr_info(&output)
                    }
                }
            }
            _ => {
                debug!("glab mr view failed, falling back to minimal PrInfo");
                fallback_mr_info(&output)
            }
        }
    }

    fn update_pr(&self, branch: &str, title: &str, body: &str) -> Result<PrInfo, PlatformError> {
        self.check_available()?;
        debug!("Updating MR for branch '{}': {}", branch, title);

        // Find MR iid via pr_exists
        let existing = self.pr_exists(branch)?;
        let iid = match existing {
            Some(ref info) => info.number,
            None => {
                return Err(PlatformError::ApiError {
                    detail: format!("No open MR found for branch '{branch}'"),
                });
            }
        };

        let iid_str = iid.to_string();
        let output = Command::new("glab")
            .args(["mr", "update", &iid_str, "--title", title, "--description", body])
            .output()
            .map_err(|e| PlatformError::Other(format!("Failed to run glab: {e}")))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            if let Some(auth_err) = Self::check_auth_error(&stderr) {
                return Err(auth_err);
            }
            return Err(PlatformError::ApiError { detail: stderr.trim().to_string() });
        }

        // Fetch updated MR info
        let view_output =
            Command::new("glab").args(["mr", "view", &iid_str, "--output", "json"]).output();

        match view_output {
            Ok(view) if view.status.success() => {
                let view_stdout = String::from_utf8_lossy(&view.stdout);
                serde_json::from_str::<GitLabMrInfo>(&view_stdout)
                    .map(PrInfo::from)
                    .map_err(|e| PlatformError::Other(format!("Failed to parse MR info: {e}")))
            }
            _ => Ok(PrInfo {
                number: iid,
                url: String::new(),
                state: "opened".to_string(),
                title: title.to_string(),
                head_branch: branch.to_string(),
                base_branch: String::new(),
            }),
        }
    }
}

fn fallback_mr_info(create_output: &std::process::Output) -> Result<PrInfo, PlatformError> {
    // `glab mr create` prints the MR URL on stdout on success
    let url = String::from_utf8_lossy(&create_output.stdout).trim().to_string();
    Ok(PrInfo {
        number: 0,
        url,
        state: "opened".to_string(),
        title: String::new(),
        head_branch: String::new(),
        base_branch: String::new(),
    })
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

    #[test]
    fn gitlab_pr_info_deserializes() {
        let json = r#"[{
            "iid": 42,
            "web_url": "https://gitlab.com/user/repo/-/merge_requests/42",
            "state": "opened",
            "title": "Add feature X",
            "source_branch": "feature-x",
            "target_branch": "main"
        }]"#;
        let mrs: Vec<GitLabMrInfo> = serde_json::from_str(json).unwrap();
        assert_eq!(mrs.len(), 1);
        let pr_info: PrInfo = mrs[0].clone().into();
        assert_eq!(pr_info.number, 42);
        assert_eq!(pr_info.url, "https://gitlab.com/user/repo/-/merge_requests/42");
        assert_eq!(pr_info.head_branch, "feature-x");
        assert_eq!(pr_info.base_branch, "main");
    }

    #[test]
    fn gitlab_empty_mr_list() {
        let json = "[]";
        let mrs: Vec<GitLabMrInfo> = serde_json::from_str(json).unwrap();
        assert!(mrs.is_empty());
    }

    #[test]
    fn availability_check_does_not_panic() {
        let _ = GitLabAdapter::is_available();
    }
}
