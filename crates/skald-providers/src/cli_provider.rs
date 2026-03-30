use std::io::Write;

use async_trait::async_trait;
use tempfile::Builder;
use tokio::process::Command;
use tracing::{debug, warn};

use crate::{CommitContext, PrContent, PrContext, Provider, ProviderError};
use crate::config::CliProviderConfig;

pub struct CliProvider {
    config: &'static CliProviderConfig,
    model: Option<String>,
}

impl CliProvider {
    pub fn new(config: &'static CliProviderConfig, model: Option<String>) -> Self {
        Self { config, model }
    }

    /// Calls the CLI binary with a prompt and optional file path references.
    ///
    /// Builds the command as: `binary prompt_args... <prompt+file_refs> tool_args... [model_flag model]`
    async fn call_cli(&self, prompt: &str, file_refs: &str) -> Result<String, ProviderError> {
        let full_prompt = if file_refs.is_empty() {
            prompt.to_string()
        } else {
            format!("{prompt}\n\n{file_refs}")
        };

        let mut cmd = Command::new(self.config.binary);

        for arg in self.config.prompt_args {
            cmd.arg(arg);
        }
        cmd.arg(&full_prompt);
        for arg in self.config.tool_args {
            cmd.arg(arg);
        }

        if let Some(ref model) = self.model {
            cmd.arg(self.config.model_flag).arg(model);
        }

        debug!(?cmd, "Spawning {} CLI", self.config.name);

        let output = cmd.output().await.map_err(|e| ProviderError::Unavailable {
            provider: self.config.name.into(),
            detail: format!("Failed to spawn {}: {e}", self.config.binary),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(%stderr, "{} CLI returned non-zero exit status", self.config.name);
            return Err(ProviderError::Generation {
                provider: self.config.name.into(),
                detail: format!("{} exited with {}: {stderr}", self.config.binary, output.status),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        debug!(len = stdout.len(), "Received {} CLI response", self.config.name);

        Ok(stdout)
    }

    /// Calls the CLI with a prompt and diff content.
    ///
    /// Writes the diff to a temp file so the CLI can read it via the Read tool,
    /// then calls `call_cli` with the file path reference.
    async fn generate_with_diff(&self, prompt: &str, diff: &str) -> Result<String, ProviderError> {
        let mut tmp = Builder::new()
            .prefix("skald-diff-")
            .suffix(".patch")
            .tempfile()
            .map_err(|e| ProviderError::Other(format!("Failed to create temp file: {e}")))?;

        tmp.write_all(diff.as_bytes())
            .map_err(|e| ProviderError::Other(format!("Failed to write diff to temp file: {e}")))?;

        let tmp_path = tmp.path().to_string_lossy().to_string();

        debug!(path = %tmp_path, "Wrote diff to temp file");

        let file_refs = format!(
            "The diff is available at: {tmp_path}\nRead that file to see the changes.",
        );

        // tmp file is dropped (and cleaned up) after call_cli returns
        self.call_cli(prompt, &file_refs).await
    }

    /// Calls the CLI with a prompt, diff, and commit log for PR generation.
    ///
    /// Writes diff and commit log to separate temp files so the CLI can read them
    /// via the Read tool.
    async fn generate_with_diff_and_log(
        &self,
        prompt: &str,
        diff: &str,
        commit_log: &str,
    ) -> Result<String, ProviderError> {
        let mut diff_tmp =
            Builder::new().prefix("skald-pr-diff-").suffix(".patch").tempfile().map_err(|e| {
                ProviderError::Other(format!("Failed to create diff temp file: {e}"))
            })?;
        diff_tmp
            .write_all(diff.as_bytes())
            .map_err(|e| ProviderError::Other(format!("Failed to write diff: {e}")))?;

        let mut log_tmp =
            Builder::new().prefix("skald-pr-log-").suffix(".txt").tempfile().map_err(|e| {
                ProviderError::Other(format!("Failed to create log temp file: {e}"))
            })?;
        log_tmp
            .write_all(commit_log.as_bytes())
            .map_err(|e| ProviderError::Other(format!("Failed to write commit log: {e}")))?;

        let diff_path = diff_tmp.path().to_string_lossy().to_string();
        let log_path = log_tmp.path().to_string_lossy().to_string();

        debug!(diff_path = %diff_path, log_path = %log_path, "Wrote PR files to temp");

        let file_refs = format!(
            "The diff is available at: {diff_path}\nThe commit log is available at: {log_path}\nRead both files to understand the full changeset.",
        );

        self.call_cli(prompt, &file_refs).await
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
impl Provider for CliProvider {
    fn name(&self) -> &str {
        self.config.name
    }

    async fn generate_commit_messages(
        &self,
        ctx: &CommitContext,
        count: usize,
    ) -> Result<Vec<String>, ProviderError> {
        let response = self.generate_with_diff(&ctx.rendered_prompt, &ctx.diff).await?;

        let messages: Vec<String> = response
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .take(count)
            .collect();

        if messages.is_empty() {
            return Err(ProviderError::Generation {
                provider: self.config.name.into(),
                detail: format!("{} returned no commit messages", self.config.name),
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
            self.generate_with_diff_and_log(&ctx.rendered_prompt, &ctx.diff, &ctx.commit_log)
                .await?;
        let contents = parse_pr_response(&response, count);
        if contents.is_empty() {
            return Err(ProviderError::Generation {
                provider: self.config.name.into(),
                detail: format!("{} returned no PR content", self.config.name),
            });
        }
        Ok(contents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::get_provider_config;

    #[test]
    fn cli_provider_name_matches_config() {
        let config = get_provider_config("claude").unwrap();
        let provider = CliProvider::new(config, None);
        assert_eq!(provider.name(), "claude");
    }

    #[test]
    fn cli_provider_with_all_configs() {
        for name in ["claude", "codex", "gemini", "opencode", "copilot"] {
            let config = get_provider_config(name).unwrap();
            let _provider = CliProvider::new(config, None);
            let _provider_with_model = CliProvider::new(config, Some("test-model".into()));
        }
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
