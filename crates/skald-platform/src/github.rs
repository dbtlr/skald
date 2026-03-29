use std::process::Command;

use tracing::debug;

use crate::{CreatePrRequest, PlatformAdapter, PlatformError, PrInfo};

pub struct GitHubAdapter;

impl GitHubAdapter {
    pub fn detect(remote_url: &str) -> Option<Self> {
        if remote_url.contains("github.com") { Some(GitHubAdapter) } else { None }
    }

    pub fn is_available() -> bool {
        Command::new("gh").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
    }

    fn check_available(&self) -> Result<(), PlatformError> {
        if !Self::is_available() {
            return Err(PlatformError::CliNotFound {
                cli: "gh".to_string(),
                install_url: "https://cli.github.com".to_string(),
            });
        }
        Ok(())
    }

    fn check_auth_error(stderr: &str) -> Option<PlatformError> {
        if stderr.contains("not logged in")
            || stderr.contains("authentication")
            || stderr.contains("HTTP 401")
            || stderr.contains("GITHUB_TOKEN")
        {
            Some(PlatformError::NotAuthenticated {
                cli: "gh".to_string(),
                auth_command: "gh auth login".to_string(),
            })
        } else {
            None
        }
    }
}

impl PlatformAdapter for GitHubAdapter {
    fn name(&self) -> &str {
        "github"
    }

    fn pr_exists(&self, branch: &str) -> Result<Option<PrInfo>, PlatformError> {
        self.check_available()?;

        debug!("Checking for existing PR for branch: {}", branch);

        let output = Command::new("gh")
            .args([
                "pr",
                "list",
                "--head",
                branch,
                "--json",
                "number,url,state,title,headRefName,baseRefName",
                "--limit",
                "1",
            ])
            .output()
            .map_err(|e| PlatformError::Other(format!("Failed to run gh: {e}")))?;

        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            if let Some(auth_err) = Self::check_auth_error(&stderr) {
                return Err(auth_err);
            }
            return Err(PlatformError::ApiError { detail: stderr.trim().to_string() });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        debug!("gh pr list output: {}", stdout.trim());

        let prs: Vec<PrInfo> = serde_json::from_str(&stdout)
            .map_err(|e| PlatformError::Other(format!("Failed to parse gh output: {e}")))?;

        Ok(prs.into_iter().next())
    }

    fn create_pr(&self, request: &CreatePrRequest) -> Result<PrInfo, PlatformError> {
        self.check_available()?;

        if request.push {
            debug!("Pushing branch to origin before creating PR");
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

        debug!("Creating PR: {}", request.title);

        let mut args = vec![
            "pr",
            "create",
            "--title",
            &request.title,
            "--body",
            &request.body,
            "--base",
            &request.base,
        ];

        if request.draft {
            args.push("--draft");
        }

        let output = Command::new("gh")
            .args(&args)
            .output()
            .map_err(|e| PlatformError::Other(format!("Failed to run gh: {e}")))?;

        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            if let Some(auth_err) = Self::check_auth_error(&stderr) {
                return Err(auth_err);
            }
            return Err(PlatformError::ApiError { detail: stderr.trim().to_string() });
        }

        // After creation, fetch structured PrInfo via `gh pr view`
        let view_output = Command::new("gh")
            .args(["pr", "view", "--json", "number,url,state,title,headRefName,baseRefName"])
            .output();

        match view_output {
            Ok(view) if view.status.success() => {
                let view_stdout = String::from_utf8_lossy(&view.stdout);
                debug!("gh pr view output: {}", view_stdout.trim());
                match serde_json::from_str::<PrInfo>(&view_stdout) {
                    Ok(info) => Ok(info),
                    Err(e) => {
                        debug!(
                            "Failed to parse gh pr view output: {e}, falling back to minimal PrInfo"
                        );
                        fallback_pr_info(&output)
                    }
                }
            }
            _ => {
                debug!("gh pr view failed, falling back to minimal PrInfo");
                fallback_pr_info(&output)
            }
        }
    }

    fn update_pr(&self, branch: &str, title: &str, body: &str) -> Result<PrInfo, PlatformError> {
        self.check_available()?;
        debug!("Updating PR for branch '{}': {}", branch, title);

        let output = Command::new("gh")
            .args(["pr", "edit", branch, "--title", title, "--body", body])
            .output()
            .map_err(|e| PlatformError::Other(format!("Failed to run gh: {e}")))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            if let Some(auth_err) = Self::check_auth_error(&stderr) {
                return Err(auth_err);
            }
            return Err(PlatformError::ApiError { detail: stderr.trim().to_string() });
        }

        // Fetch updated PR info
        let view_output = Command::new("gh")
            .args([
                "pr",
                "view",
                branch,
                "--json",
                "number,url,state,title,headRefName,baseRefName",
            ])
            .output();

        match view_output {
            Ok(view) if view.status.success() => {
                let view_stdout = String::from_utf8_lossy(&view.stdout);
                serde_json::from_str::<PrInfo>(&view_stdout)
                    .map_err(|e| PlatformError::Other(format!("Failed to parse PR info: {e}")))
            }
            _ => Ok(PrInfo {
                number: 0,
                url: String::new(),
                state: "open".to_string(),
                title: title.to_string(),
                head_branch: branch.to_string(),
                base_branch: String::new(),
            }),
        }
    }
}

fn fallback_pr_info(create_output: &std::process::Output) -> Result<PrInfo, PlatformError> {
    // `gh pr create` prints the PR URL on stdout on success
    let url = String::from_utf8_lossy(&create_output.stdout).trim().to_string();
    Ok(PrInfo {
        number: 0,
        url,
        state: "open".to_string(),
        title: String::new(),
        head_branch: String::new(),
        base_branch: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_https_url() {
        let result = GitHubAdapter::detect("https://github.com/user/repo.git");
        assert!(result.is_some());
    }

    #[test]
    fn detect_ssh_url() {
        let result = GitHubAdapter::detect("git@github.com:user/repo.git");
        assert!(result.is_some());
    }

    #[test]
    fn detect_https_no_git_suffix() {
        let result = GitHubAdapter::detect("https://github.com/user/repo");
        assert!(result.is_some());
    }

    #[test]
    fn detect_non_github_returns_none() {
        assert!(GitHubAdapter::detect("https://gitlab.com/user/repo.git").is_none());
        assert!(GitHubAdapter::detect("https://bitbucket.org/user/repo.git").is_none());
    }

    #[test]
    fn pr_info_deserializes_from_gh_json() {
        let json = r#"[
            {
                "number": 42,
                "url": "https://github.com/user/repo/pull/42",
                "state": "OPEN",
                "title": "feat: add thing",
                "headRefName": "feat/add-thing",
                "baseRefName": "main"
            }
        ]"#;
        let prs: Vec<PrInfo> = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(prs.len(), 1);
        let pr = &prs[0];
        assert_eq!(pr.number, 42);
        assert_eq!(pr.url, "https://github.com/user/repo/pull/42");
        assert_eq!(pr.state, "OPEN");
        assert_eq!(pr.title, "feat: add thing");
        assert_eq!(pr.head_branch, "feat/add-thing");
        assert_eq!(pr.base_branch, "main");
    }

    #[test]
    fn pr_info_empty_array() {
        let json = r#"[]"#;
        let prs: Vec<PrInfo> = serde_json::from_str(json).expect("should deserialize");
        assert!(prs.is_empty());
    }

    #[test]
    fn availability_check_does_not_panic() {
        let _ = GitHubAdapter::is_available();
    }

    #[test]
    fn update_pr_method_exists() {
        let adapter = GitHubAdapter;
        assert_eq!(adapter.name(), "github");
        // Compilation verifies the method exists on the trait
    }
}
