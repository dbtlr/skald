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

    async fn generate_pr_content(&self, _ctx: &PrContext) -> Result<Vec<PrContent>, ProviderError> {
        Err(ProviderError::Other("PR generation not yet implemented".into()))
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
}
