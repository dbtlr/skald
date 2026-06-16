//! Structured-output types shared by the API providers (Anthropic, Codex).
//!
//! Both providers ask the model for JSON matching these schemas, so the shapes
//! live in one place and the JSON Schema is generated identically. The
//! parse-with-fallback helpers below are shared too: each provider calls them,
//! then maps an empty result onto its own provider-named error.

use serde::Deserialize;

use crate::providers::PrContent;

/// Commit-message generation response: a list of candidate messages.
///
/// `deny_unknown_fields` makes schemars emit `additionalProperties: false`, which
/// OpenAI/Codex strict `json_schema` mode requires on every object.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommitResponse {
    pub messages: Vec<String>,
}

/// PR generation response: a list of title/body candidates.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrResponse {
    pub entries: Vec<PrEntry>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrEntry {
    pub title: String,
    pub body: String,
}

/// JSON Schema for a structured-output type, as a `serde_json::Value`.
pub fn schema_value<T: schemars::JsonSchema>() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(T)).expect("schema serialization failed")
}

/// Parse commit messages from a model's text response.
///
/// Tries the structured `CommitResponse` JSON first; on any parse failure,
/// falls back to treating each non-empty line as a candidate message. Returns
/// at most `count` messages — possibly empty, which the caller maps onto its
/// own provider-named error.
pub fn parse_commit_messages(text: &str, count: usize) -> Vec<String> {
    match serde_json::from_str::<CommitResponse>(text) {
        Ok(parsed) => parsed
            .messages
            .into_iter()
            .take(count)
            .map(|m| m.trim().to_string())
            .filter(|m| !m.is_empty())
            .collect(),
        Err(_) => {
            tracing::debug!("structured parse failed, falling back to line-based parsing");
            text.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .take(count)
                .collect()
        }
    }
}

/// Parse PR title/body candidates from a model's text response.
///
/// Tries the structured `PrResponse` JSON first; on any parse failure, falls
/// back to the CLI provider's text-based `parse_pr_response`. Returns at most
/// `count` entries — possibly empty, which the caller maps onto its own
/// provider-named error.
pub fn parse_pr_entries(text: &str, count: usize) -> Vec<PrContent> {
    match serde_json::from_str::<PrResponse>(text) {
        Ok(parsed) => parsed
            .entries
            .into_iter()
            .take(count)
            .map(|e| PrContent {
                title: e.title.trim().to_string(),
                body: e.body.trim().to_string(),
            })
            .collect(),
        Err(_) => {
            tracing::debug!("structured parse failed, falling back to text-based parsing");
            crate::providers::cli_provider::parse_pr_response(text, count)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_schema_describes_messages_array() {
        let schema = schema_value::<CommitResponse>();
        let props = schema.get("properties").expect("schema has properties");
        let messages = props.get("messages").expect("schema has messages property");
        assert_eq!(messages.get("type").and_then(|v| v.as_str()), Some("array"));
    }

    #[test]
    fn schemas_forbid_additional_properties_for_strict_mode() {
        // OpenAI/Codex strict json_schema mode requires every object to set
        // additionalProperties:false — including nested definitions.
        let commit = schema_value::<CommitResponse>();
        assert_eq!(commit.get("additionalProperties"), Some(&serde_json::json!(false)));

        let pr = schema_value::<PrResponse>();
        assert_eq!(pr.get("additionalProperties"), Some(&serde_json::json!(false)));
        // The nested PrEntry definition must also forbid extra properties.
        let entry = schema_value::<PrEntry>();
        assert_eq!(entry.get("additionalProperties"), Some(&serde_json::json!(false)));
    }

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
        let json = r#"{"entries": [{"title": "Add auth", "body": "What\nAuth system"}]}"#;
        let parsed: PrResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].title, "Add auth");
        assert!(parsed.entries[0].body.contains("Auth system"));
    }

    #[test]
    fn parse_commit_messages_from_structured_json() {
        let text = r#"{"messages": ["feat: a", "fix: b", "chore: c"]}"#;
        let messages = parse_commit_messages(text, 2);
        assert_eq!(messages, vec!["feat: a", "fix: b"]);
    }

    #[test]
    fn parse_commit_messages_falls_back_to_lines() {
        let text = "feat: a\n\n  fix: b  \nchore: c";
        let messages = parse_commit_messages(text, 10);
        assert_eq!(messages, vec!["feat: a", "fix: b", "chore: c"]);
    }

    #[test]
    fn parse_commit_messages_empty_on_blank_text() {
        assert!(parse_commit_messages("   \n  \n", 3).is_empty());
    }

    #[test]
    fn parse_pr_entries_from_structured_json() {
        let text = r#"{"entries": [{"title": "Add auth", "body": "Auth system"}]}"#;
        let entries = parse_pr_entries(text, 5);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Add auth");
        assert_eq!(entries[0].body, "Auth system");
    }

    #[test]
    fn parse_pr_entries_respects_count() {
        let text = r#"{"entries": [{"title": "a", "body": "x"}, {"title": "b", "body": "y"}, {"title": "c", "body": "z"}]}"#;
        let entries = parse_pr_entries(text, 2);
        assert_eq!(entries.len(), 2);
    }
}
