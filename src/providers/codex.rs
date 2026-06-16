//! Codex API provider — reuses a ChatGPT subscription's OAuth login
//! (`~/.codex/auth.json`) to call the Codex inference backend directly, with no
//! `codex` binary shell-out.
//!
//! The backend (`chatgpt.com/backend-api/codex`) speaks the OpenAI **Responses**
//! API and is **streaming-only** (SSE). We consume the stream internally and
//! return the assembled text; commit/PR generation is one-shot, so there is no
//! user-facing streaming.

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::engine::compaction::compact_diff;
use crate::providers::codex_auth::{CodexCreds, load_codex_creds};
use crate::providers::structured::{
    CommitResponse, PrResponse, parse_commit_messages, parse_pr_entries, schema_value,
};
use crate::providers::{CommitContext, PrContent, PrContext, Provider, ProviderError};

/// Codex inference backend (ChatGPT-subscription auth).
pub const DEFAULT_CODEX_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";
/// Default model for the codex-api provider.
pub const DEFAULT_CODEX_MODEL: &str = "gpt-5.5";

pub struct CodexProvider {
    creds: CodexCreds,
    model: String,
    base_url: String,
}

impl CodexProvider {
    /// Build a provider from the on-disk Codex subscription login.
    pub fn from_codex_auth(model: String, base_url: Option<String>) -> Result<Self, ProviderError> {
        let creds = load_codex_creds().map_err(|e| ProviderError::Unavailable {
            provider: "codex-api".to_string(),
            detail: e.to_string(),
        })?;
        Ok(Self {
            creds,
            model,
            base_url: base_url.unwrap_or_else(|| DEFAULT_CODEX_BASE_URL.to_string()),
        })
    }
}

/// Assemble the model's text output from a Codex Responses SSE stream body.
///
/// Concatenates the `delta` field of every `response.output_text.delta` event,
/// ignoring all other event types, blank lines, and the `[DONE]` sentinel.
pub fn assemble_sse_text(body: &str) -> String {
    let mut out = String::new();
    for line in body.lines() {
        let Some(payload) = line.strip_prefix("data:") else {
            continue;
        };
        let payload = payload.trim();
        if payload.is_empty() || payload == "[DONE]" {
            continue;
        }
        let Ok(event) = serde_json::from_str::<Value>(payload) else {
            continue;
        };
        if event.get("type").and_then(|t| t.as_str()) == Some("response.output_text.delta")
            && let Some(delta) = event.get("delta").and_then(|d| d.as_str())
        {
            out.push_str(delta);
        }
    }
    out
}

/// Build the Responses-API request body for a structured (JSON-schema) call.
pub fn build_responses_body(
    model: &str,
    instructions: &str,
    user_text: &str,
    schema_name: &str,
    schema: Value,
    effort: &str,
) -> Value {
    json!({
        "model": model,
        "instructions": instructions,
        "input": [{
            "role": "user",
            "content": [{ "type": "input_text", "text": user_text }],
        }],
        "store": false,
        "stream": true,
        "reasoning": { "effort": effort },
        "text": {
            "format": {
                "type": "json_schema",
                "name": schema_name,
                "strict": true,
                "schema": schema,
            }
        },
    })
}

/// Best-effort total token count from the `response.completed` SSE event.
pub fn total_tokens_from_sse(body: &str) -> Option<u64> {
    for line in body.lines() {
        let Some(payload) = line.strip_prefix("data:") else {
            continue;
        };
        let payload = payload.trim();
        if payload.is_empty() || payload == "[DONE]" {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<Value>(payload)
            && event.get("type").and_then(|t| t.as_str()) == Some("response.completed")
        {
            return event
                .get("response")
                .and_then(|r| r.get("usage"))
                .and_then(|u| u.get("total_tokens"))
                .and_then(|t| t.as_u64());
        }
    }
    None
}

/// Blocking HTTP call to the Codex Responses endpoint. Returns the raw SSE body.
fn call_codex_blocking(
    base_url: &str,
    token: &str,
    account_id: &str,
    body: Value,
) -> Result<String, ProviderError> {
    let url = format!("{}/responses", base_url.trim_end_matches('/'));
    let result = ureq::post(&url)
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json")
        .set("Accept", "text/event-stream")
        .set("originator", "codex_cli_rs")
        .set("User-Agent", "codex_cli_rs/0.0.0 (skald)")
        .set("ChatGPT-Account-ID", account_id)
        .send_json(body);

    match result {
        Ok(resp) => resp.into_string().map_err(|e| ProviderError::Generation {
            provider: "codex-api".to_string(),
            detail: format!("Failed to read Codex response stream: {e}"),
        }),
        Err(ureq::Error::Status(code, resp)) => {
            let detail = resp.into_string().unwrap_or_default();
            Err(map_status_error(code, &detail))
        }
        Err(e) => Err(ProviderError::Unavailable {
            provider: "codex-api".to_string(),
            detail: format!("Could not reach the Codex backend: {e}"),
        }),
    }
}

fn map_status_error(code: u16, detail: &str) -> ProviderError {
    let provider = "codex-api".to_string();
    match code {
        401 | 403 => ProviderError::Unavailable {
            provider,
            detail: "Codex login expired or unauthorized. Run `codex` to re-authenticate."
                .to_string(),
        },
        429 => ProviderError::Generation {
            provider,
            detail: "Codex backend rate-limited or quota exhausted. Your login is still valid — retry after the limit resets."
                .to_string(),
        },
        _ => ProviderError::Generation {
            provider,
            detail: format!("Codex backend returned {code}: {}", detail.trim()),
        },
    }
}

impl CodexProvider {
    /// Run one structured Responses call and return the assembled output text.
    async fn run(
        &self,
        instructions: &str,
        user_text: &str,
        schema_name: &str,
        schema: Value,
    ) -> Result<String, ProviderError> {
        let body =
            build_responses_body(&self.model, instructions, user_text, schema_name, schema, "low");
        let base_url = self.base_url.clone();
        let token = self.creds.access_token.clone();
        let account_id = self.creds.account_id.clone();

        let sse = tokio::task::spawn_blocking(move || {
            call_codex_blocking(&base_url, &token, &account_id, body)
        })
        .await
        .map_err(|e| ProviderError::Other(format!("Codex request task failed: {e}")))??;

        if let Some(total) = total_tokens_from_sse(&sse) {
            tracing::info!(total_tokens = total, "Codex API token usage");
        }
        Ok(assemble_sse_text(&sse))
    }
}

#[async_trait]
impl Provider for CodexProvider {
    fn name(&self) -> &str {
        "codex-api"
    }

    async fn generate_commit_messages(
        &self,
        ctx: &CommitContext,
        count: usize,
    ) -> Result<Vec<String>, ProviderError> {
        let compacted = compact_diff(&ctx.diff, &ctx.stat);
        if compacted.was_compacted {
            tracing::info!(dropped = ?compacted.dropped_files, "diff compacted before sending to Codex API");
        }
        let user_message = format!(
            "{}\n\n## Diff\n\n```\n{}\n```\n\n## Diff Stat\n\n```\n{}\n```",
            ctx.rendered_prompt, compacted.diff, compacted.stat
        );

        let text = self
            .run(
                &ctx.rendered_prompt,
                &user_message,
                "commit_response",
                schema_value::<CommitResponse>(),
            )
            .await?;

        let messages = parse_commit_messages(&text, count);

        if messages.is_empty() {
            return Err(ProviderError::Generation {
                provider: "codex-api".to_string(),
                detail: "Codex API returned no commit messages".to_string(),
            });
        }
        Ok(messages)
    }

    async fn generate_pr_content(
        &self,
        ctx: &PrContext,
        count: usize,
    ) -> Result<Vec<PrContent>, ProviderError> {
        let compacted = compact_diff(&ctx.diff, &ctx.diff);
        if compacted.was_compacted {
            tracing::info!(dropped = ?compacted.dropped_files, "diff compacted before sending to Codex API");
        }
        let user_message = format!(
            "{}\n\n## Diff\n\n```\n{}\n```\n\n## Commit Log\n\n```\n{}\n```",
            ctx.rendered_prompt, compacted.diff, ctx.commit_log
        );

        let text = self
            .run(&ctx.rendered_prompt, &user_message, "pr_response", schema_value::<PrResponse>())
            .await?;

        let entries = parse_pr_entries(&text, count);

        if entries.is_empty() {
            return Err(ProviderError::Generation {
                provider: "codex-api".to_string(),
                detail: "Codex API returned no PR content".to_string(),
            });
        }
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_sse() -> &'static str {
        "event: response.created\n\
         data: {\"type\":\"response.created\"}\n\
         \n\
         event: response.output_text.delta\n\
         data: {\"type\":\"response.output_text.delta\",\"delta\":\"feat(auth): \"}\n\
         \n\
         event: response.output_text.delta\n\
         data: {\"type\":\"response.output_text.delta\",\"delta\":\"add login\"}\n\
         \n\
         event: response.completed\n\
         data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\"}}\n\
         \n\
         data: [DONE]\n"
    }

    #[test]
    fn assembles_output_text_deltas_in_order() {
        assert_eq!(assemble_sse_text(sample_sse()), "feat(auth): add login");
    }

    #[test]
    fn ignores_non_delta_events_and_done_sentinel() {
        let sse = "event: response.created\n\
                   data: {\"type\":\"response.created\"}\n\
                   \n\
                   data: [DONE]\n";
        assert_eq!(assemble_sse_text(sse), "");
    }

    #[test]
    fn assembled_text_parses_as_structured_json() {
        let sse = "data: {\"type\":\"response.output_text.delta\",\"delta\":\"{\\\"messages\\\":[\\\"feat: x\\\"]}\"}\n";
        let text = assemble_sse_text(sse);
        let parsed: CommitResponse = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed.messages, vec!["feat: x"]);
    }

    #[test]
    fn extracts_total_tokens_from_completed_event() {
        let sse = "data: {\"type\":\"response.completed\",\"response\":{\"usage\":{\"total_tokens\":37}}}\n";
        assert_eq!(total_tokens_from_sse(sse), Some(37));
    }

    #[test]
    fn extracts_total_tokens_from_realistic_stream_with_event_lines() {
        // Real streams interleave `event:` header lines and blank lines before
        // the `data:` payloads — usage must still be found.
        let sse = "event: response.created\n\
                   data: {\"type\":\"response.created\"}\n\
                   \n\
                   event: response.completed\n\
                   data: {\"type\":\"response.completed\",\"response\":{\"usage\":{\"total_tokens\":37}}}\n";
        assert_eq!(total_tokens_from_sse(sse), Some(37));
    }

    #[test]
    fn no_usage_when_no_completed_event() {
        let sse = "data: {\"type\":\"response.output_text.delta\",\"delta\":\"x\"}\n";
        assert_eq!(total_tokens_from_sse(sse), None);
    }

    #[test]
    fn maps_401_to_relogin_guidance() {
        let err = map_status_error(401, "unauthorized");
        match err {
            ProviderError::Unavailable { detail, .. } => assert!(detail.contains("codex")),
            _ => panic!("401 should map to Unavailable"),
        }
    }

    #[test]
    fn maps_429_to_retry_guidance_keeping_creds() {
        let err = map_status_error(429, "rate limited");
        match err {
            ProviderError::Generation { detail, .. } => assert!(detail.contains("retry")),
            _ => panic!("429 should map to Generation"),
        }
    }

    #[test]
    fn body_is_streaming_non_stored_responses_request() {
        let body = build_responses_body(
            "gpt-5.5",
            "system prompt",
            "user diff",
            "commit_response",
            schema_value::<CommitResponse>(),
            "low",
        );
        assert_eq!(body["model"], "gpt-5.5");
        assert_eq!(body["stream"], true);
        assert_eq!(body["store"], false);
        assert_eq!(body["instructions"], "system prompt");
        assert_eq!(body["text"]["format"]["type"], "json_schema");
        assert_eq!(body["text"]["format"]["name"], "commit_response");
        // user text is carried as an input_text part on a user message
        assert_eq!(body["input"][0]["role"], "user");
        assert_eq!(body["input"][0]["content"][0]["type"], "input_text");
        assert_eq!(body["input"][0]["content"][0]["text"], "user diff");
    }
}
