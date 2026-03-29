use std::io::Write;

use async_trait::async_trait;
use tempfile::Builder;
use tokio::process::Command;
use tracing::{debug, warn};

use crate::{CommitContext, PrContent, PrContext, Provider, ProviderError};

pub struct ClaudeCliProvider {
    pub model: Option<String>,
}

impl ClaudeCliProvider {
    pub fn new(model: Option<String>) -> Self {
        Self { model }
    }

    /// Checks whether the `claude` CLI binary is available on PATH.
    pub fn is_available() -> bool {
        std::process::Command::new("claude")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Calls Claude CLI with a prompt and diff content.
    ///
    /// Writes the diff to a temp file so Claude can read it via the Read tool,
    /// then spawns `claude -p <prompt> --allowedTools Read`.
    async fn call_claude(&self, prompt: &str, diff: &str) -> Result<String, ProviderError> {
        let mut tmp = Builder::new()
            .prefix("skald-diff-")
            .suffix(".patch")
            .tempfile()
            .map_err(|e| ProviderError::Other(format!("Failed to create temp file: {e}")))?;

        tmp.write_all(diff.as_bytes())
            .map_err(|e| ProviderError::Other(format!("Failed to write diff to temp file: {e}")))?;

        let tmp_path = tmp.path().to_string_lossy().to_string();

        let full_prompt = format!(
            "{prompt}\n\nThe diff is available at: {tmp_path}\nRead that file to see the changes.",
        );

        debug!(path = %tmp_path, "Wrote diff to temp file");

        let mut cmd = Command::new("claude");
        cmd.arg("-p").arg(&full_prompt).arg("--allowedTools").arg("Read");

        if let Some(ref model) = self.model {
            cmd.arg("--model").arg(model);
        }

        debug!(?cmd, "Spawning claude CLI");

        let output = cmd.output().await.map_err(|e| ProviderError::Unavailable {
            provider: "claude-cli".into(),
            detail: format!("Failed to spawn claude: {e}"),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(%stderr, "claude CLI returned non-zero exit status");
            return Err(ProviderError::Generation {
                provider: "claude-cli".into(),
                detail: format!("claude exited with {}: {stderr}", output.status),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        debug!(len = stdout.len(), "Received claude CLI response");

        // tmp file is dropped (and cleaned up) here
        Ok(stdout)
    }

    /// Calls Claude CLI with a prompt, diff, and commit log for PR generation.
    ///
    /// Writes diff and commit log to separate temp files so Claude can read them
    /// via the Read tool, then spawns `claude -p <prompt> --allowedTools Read`.
    async fn call_claude_pr(
        &self,
        prompt: &str,
        diff: &str,
        commit_log: &str,
    ) -> Result<String, ProviderError> {
        // Write diff to temp file
        let mut diff_tmp =
            Builder::new().prefix("skald-pr-diff-").suffix(".patch").tempfile().map_err(|e| {
                ProviderError::Other(format!("Failed to create diff temp file: {e}"))
            })?;
        diff_tmp
            .write_all(diff.as_bytes())
            .map_err(|e| ProviderError::Other(format!("Failed to write diff: {e}")))?;

        // Write commit log to temp file
        let mut log_tmp =
            Builder::new().prefix("skald-pr-log-").suffix(".txt").tempfile().map_err(|e| {
                ProviderError::Other(format!("Failed to create log temp file: {e}"))
            })?;
        log_tmp
            .write_all(commit_log.as_bytes())
            .map_err(|e| ProviderError::Other(format!("Failed to write commit log: {e}")))?;

        let diff_path = diff_tmp.path().to_string_lossy().to_string();
        let log_path = log_tmp.path().to_string_lossy().to_string();

        let full_prompt = format!(
            "{prompt}\n\nThe diff is available at: {diff_path}\nThe commit log is available at: {log_path}\nRead both files to understand the full changeset.",
        );

        debug!(diff_path = %diff_path, log_path = %log_path, "Wrote PR files to temp");

        let mut cmd = Command::new("claude");
        cmd.arg("-p").arg(&full_prompt).arg("--allowedTools").arg("Read");
        if let Some(ref model) = self.model {
            cmd.arg("--model").arg(model);
        }

        debug!(?cmd, "Spawning claude CLI for PR generation");

        let output = cmd.output().await.map_err(|e| ProviderError::Unavailable {
            provider: "claude-cli".into(),
            detail: format!("Failed to spawn claude: {e}"),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(%stderr, "claude CLI returned non-zero exit status");
            return Err(ProviderError::Generation {
                provider: "claude-cli".into(),
                detail: format!("claude exited with {}: {stderr}", output.status),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

/// Parse AI response into PR title + body pairs.
/// First line is title, rest is body. Multiple candidates separated by "---".
fn parse_pr_response(response: &str, count: usize) -> Vec<PrContent> {
    let response = response.trim();
    if response.is_empty() {
        return vec![];
    }

    let candidates: Vec<&str> =
        if count > 1 { response.split("\n---\n").collect() } else { vec![response] };

    candidates
        .into_iter()
        .filter_map(|candidate| {
            let candidate = candidate.trim();
            if candidate.is_empty() {
                return None;
            }
            let mut lines = candidate.lines();
            let title = lines.next()?.trim().to_string();
            if title.is_empty() {
                return None;
            }
            let body: String = lines.collect::<Vec<_>>().join("\n").trim().to_string();
            Some(PrContent { title, body })
        })
        .take(count)
        .collect()
}

#[async_trait]
impl Provider for ClaudeCliProvider {
    fn name(&self) -> &str {
        "claude-cli"
    }

    async fn generate_commit_messages(
        &self,
        ctx: &CommitContext,
        count: usize,
    ) -> Result<Vec<String>, ProviderError> {
        let response = self.call_claude(&ctx.rendered_prompt, &ctx.diff).await?;

        let messages: Vec<String> = response
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .take(count)
            .collect();

        if messages.is_empty() {
            return Err(ProviderError::Generation {
                provider: "claude-cli".into(),
                detail: "Claude returned no commit messages".into(),
            });
        }

        Ok(messages)
    }

    async fn generate_pr_content(
        &self,
        ctx: &PrContext,
        count: usize,
    ) -> Result<Vec<PrContent>, ProviderError> {
        let response =
            self.call_claude_pr(&ctx.rendered_prompt, &ctx.diff, &ctx.commit_log).await?;
        let contents = parse_pr_response(&response, count);
        if contents.is_empty() {
            return Err(ProviderError::Generation {
                provider: "claude-cli".into(),
                detail: "Claude returned no PR content".into(),
            });
        }
        Ok(contents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn availability_check_does_not_panic() {
        let _ = ClaudeCliProvider::is_available();
    }

    #[test]
    fn new_with_model() {
        let p = ClaudeCliProvider::new(Some("claude-sonnet-4".into()));
        assert_eq!(p.name(), "claude-cli");
        assert_eq!(p.model, Some("claude-sonnet-4".into()));
    }

    #[test]
    fn new_without_model() {
        let p = ClaudeCliProvider::new(None);
        assert!(p.model.is_none());
    }

    #[test]
    fn parse_pr_response_single() {
        let response = "Add OAuth2 token refresh\n\n## What\nThis PR adds OAuth2.\n\n## Why\nUsers getting logged out.";
        let contents = parse_pr_response(response, 1);
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].title, "Add OAuth2 token refresh");
        assert!(contents[0].body.contains("## What"));
    }

    #[test]
    fn parse_pr_response_multiple() {
        let response =
            "First title\n\n## What\nFirst body\n\n---\n\nSecond title\n\n## What\nSecond body";
        let contents = parse_pr_response(response, 2);
        assert_eq!(contents.len(), 2);
        assert_eq!(contents[0].title, "First title");
        assert_eq!(contents[1].title, "Second title");
    }

    #[test]
    fn parse_pr_response_empty() {
        let contents = parse_pr_response("", 1);
        assert!(contents.is_empty());
    }

    #[test]
    fn parse_pr_response_trims_whitespace() {
        let response = "\n  Some Title  \n\n  Body text  \n";
        let contents = parse_pr_response(response, 1);
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].title, "Some Title");
        assert_eq!(contents[0].body, "Body text");
    }
}
