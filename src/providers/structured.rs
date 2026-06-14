//! Structured-output types shared by the API providers (Anthropic, Codex).
//!
//! Both providers ask the model for JSON matching these schemas, so the shapes
//! live in one place and the JSON Schema is generated identically.

use serde::Deserialize;

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
}
