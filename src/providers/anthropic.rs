use async_trait::async_trait;
use genai::Client;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, JsonSpec};
use genai::resolver::{AuthData, AuthResolver, Endpoint, ServiceTargetResolver};

use crate::engine::compaction::compact_diff;
use crate::providers::structured::{
    CommitResponse, PrResponse, parse_commit_messages, parse_pr_entries, schema_value,
};
use crate::providers::{CommitContext, PrContent, PrContext, Provider, ProviderError};

/// Direct Anthropic API provider using genai SDK.
pub struct AnthropicProvider {
    client: Client,
    model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, base_url: Option<String>, model: String) -> Self {
        let mut builder = Client::builder().with_auth_resolver(AuthResolver::from_resolver_fn(
            move |_model_iden| Ok(Some(AuthData::from_single(api_key.clone()))),
        ));

        if let Some(url) = base_url {
            builder = builder.with_service_target_resolver(
                ServiceTargetResolver::from_resolver_fn(move |mut target: genai::ServiceTarget| {
                    if target.model.adapter_kind == genai::adapter::AdapterKind::Anthropic {
                        target.endpoint = Endpoint::from_owned(url.clone());
                    }
                    Ok(target)
                }),
            );
        }

        let client = builder.build();
        Self { client, model }
    }
}

fn json_spec_from<T: schemars::JsonSchema>(name: &str) -> JsonSpec {
    JsonSpec::new(name, schema_value::<T>())
}

fn map_genai_error(e: genai::Error) -> ProviderError {
    let detail = format!("{e}");

    if detail.contains("401") || detail.contains("authentication") || detail.contains("api_key") {
        return ProviderError::Generation {
            provider: "anthropic".to_string(),
            detail: "Authentication failed. Check your API key is valid.".to_string(),
        };
    }

    if detail.contains("429") || detail.contains("rate") {
        return ProviderError::Generation {
            provider: "anthropic".to_string(),
            detail:
                "Rate limited by Anthropic. Wait a moment and retry, or check your plan limits."
                    .to_string(),
        };
    }

    if detail.contains("529") || detail.contains("overloaded") {
        return ProviderError::Generation {
            provider: "anthropic".to_string(),
            detail: "Anthropic API is overloaded. Try again shortly.".to_string(),
        };
    }

    if detail.contains("context") || detail.contains("too long") || detail.contains("max_tokens") {
        return ProviderError::Generation {
            provider: "anthropic".to_string(),
            detail: "Diff is too large even after compaction. Try staging fewer files or use a model with a larger context window.".to_string(),
        };
    }

    ProviderError::Generation { provider: "anthropic".to_string(), detail }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn generate_commit_messages(
        &self,
        ctx: &CommitContext,
        count: usize,
    ) -> Result<Vec<String>, ProviderError> {
        let compacted = compact_diff(&ctx.diff, &ctx.stat);
        if compacted.was_compacted {
            tracing::info!(dropped = ?compacted.dropped_files, "diff compacted before sending to API");
        }

        let user_message = format!(
            "{}\n\n## Diff\n\n```\n{}\n```\n\n## Diff Stat\n\n```\n{}\n```",
            ctx.rendered_prompt, compacted.diff, compacted.stat
        );

        let chat_req = ChatRequest::default()
            .with_system(&ctx.rendered_prompt)
            .append_message(ChatMessage::user(user_message));

        let opts = ChatOptions::default()
            .with_response_format(json_spec_from::<CommitResponse>("commit_response"));

        let response = self
            .client
            .exec_chat(&self.model, chat_req, Some(&opts))
            .await
            .map_err(map_genai_error)?;

        let text = response.first_text().ok_or_else(|| ProviderError::Generation {
            provider: "anthropic".to_string(),
            detail: "Empty response from API".to_string(),
        })?;

        tracing::info!(
            prompt_tokens = response.usage.prompt_tokens.unwrap_or(0),
            completion_tokens = response.usage.completion_tokens.unwrap_or(0),
            total_tokens = response.usage.total_tokens.unwrap_or(0),
            "API token usage"
        );

        let messages = parse_commit_messages(text, count);
        if messages.is_empty() {
            return Err(ProviderError::Generation {
                provider: "anthropic".to_string(),
                detail: "API returned no commit messages".to_string(),
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
            tracing::info!(dropped = ?compacted.dropped_files, "diff compacted before sending to API");
        }

        let user_message = format!(
            "{}\n\n## Diff\n\n```\n{}\n```\n\n## Commit Log\n\n```\n{}\n```",
            ctx.rendered_prompt, compacted.diff, ctx.commit_log
        );

        let chat_req = ChatRequest::default()
            .with_system(&ctx.rendered_prompt)
            .append_message(ChatMessage::user(user_message));

        let opts = ChatOptions::default()
            .with_response_format(json_spec_from::<PrResponse>("pr_response"));

        let response = self
            .client
            .exec_chat(&self.model, chat_req, Some(&opts))
            .await
            .map_err(map_genai_error)?;

        let text = response.first_text().ok_or_else(|| ProviderError::Generation {
            provider: "anthropic".to_string(),
            detail: "Empty response from API".to_string(),
        })?;

        tracing::info!(
            prompt_tokens = response.usage.prompt_tokens.unwrap_or(0),
            completion_tokens = response.usage.completion_tokens.unwrap_or(0),
            total_tokens = response.usage.total_tokens.unwrap_or(0),
            "API token usage"
        );

        let entries = parse_pr_entries(text, count);
        if entries.is_empty() {
            return Err(ProviderError::Generation {
                provider: "anthropic".to_string(),
                detail: "API returned no PR content".to_string(),
            });
        }
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_commit_response_json() {
        let json =
            r#"{"messages": ["feat(auth): add login endpoint", "feat: implement auth flow"]}"#;
        let parsed: CommitResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.messages.len(), 2);
        assert_eq!(parsed.messages[0], "feat(auth): add login endpoint");
    }

    #[test]
    fn parse_pr_response_json() {
        let json = "{\"entries\": [{\"title\": \"Add auth\", \"body\": \"What\\nAuth system\"}, {\"title\": \"Auth flow\", \"body\": \"What\\nLogin\"}]}";
        let parsed: PrResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].title, "Add auth");
        assert!(parsed.entries[0].body.contains("Auth system"));
    }

    #[test]
    fn parse_commit_response_empty_messages() {
        let json = r#"{"messages": []}"#;
        let parsed: CommitResponse = serde_json::from_str(json).unwrap();
        assert!(parsed.messages.is_empty());
    }

    #[test]
    fn json_spec_generates_valid_schema() {
        let spec = json_spec_from::<CommitResponse>("test");
        // Verify it doesn't panic — schema generation works
        let _ = spec;
    }
}
