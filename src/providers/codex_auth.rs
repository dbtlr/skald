//! Reads Codex / ChatGPT subscription credentials from `~/.codex/auth.json`.
//!
//! The Codex CLI stores OAuth credentials at `~/.codex/auth.json` (or
//! `$CODEX_HOME/auth.json`). When the user signed in with a ChatGPT
//! subscription (`auth_mode: "chatgpt"`), we reuse those tokens to call the
//! Codex inference backend directly — no `codex` binary shell-out.
//!
//! This is **read-only**: we never write back to the shared file. OpenAI refresh
//! tokens are single-use/rotating, so refreshing in place would invalidate the
//! Codex CLI's copy. Expired tokens surface as an HTTP 401 at call time.

use std::path::PathBuf;

/// Credentials extracted from a ChatGPT-mode `~/.codex/auth.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexCreds {
    /// OAuth access token, used as the `Authorization: Bearer` value.
    pub access_token: String,
    /// ChatGPT account id, sent as the `ChatGPT-Account-ID` header.
    pub account_id: String,
}

/// Why we couldn't produce usable Codex subscription credentials.
#[derive(Debug, thiserror::Error)]
pub enum CodexAuthError {
    #[error(
        "Codex subscription login not found at {0}.\n\nRun `codex` to sign in, or use a provider with an API key (e.g. --provider anthropic)."
    )]
    NotFound(PathBuf),

    #[error(
        "Codex auth at {0} is in API-key mode, not ChatGPT subscription mode.\n\nThe `codex-api` provider needs a ChatGPT login — run `codex` and sign in with ChatGPT."
    )]
    NotChatgptMode(PathBuf),

    #[error("Codex auth file is malformed: {0}")]
    Malformed(String),

    #[error("Codex auth is missing an access token.\n\nRun `codex` to re-authenticate.")]
    MissingToken,
}

/// Path to the Codex auth file: `$CODEX_HOME/auth.json`, else `~/.codex/auth.json`.
pub fn codex_auth_path() -> PathBuf {
    if let Ok(home) = std::env::var("CODEX_HOME")
        && !home.is_empty()
    {
        return PathBuf::from(home).join("auth.json");
    }
    dirs::home_dir().unwrap_or_default().join(".codex").join("auth.json")
}

/// Parse the contents of a Codex `auth.json` into usable credentials.
///
/// Requires `auth_mode == "chatgpt"` and a non-empty `tokens.access_token`.
/// `tokens.account_id` is read directly (it matches the JWT's
/// `chatgpt_account_id` claim, so no JWT decoding is needed).
pub fn parse_codex_auth(contents: &str) -> Result<CodexCreds, CodexAuthError> {
    let root: serde_json::Value =
        serde_json::from_str(contents).map_err(|e| CodexAuthError::Malformed(e.to_string()))?;

    if root.get("auth_mode").and_then(|v| v.as_str()) != Some("chatgpt") {
        return Err(CodexAuthError::NotChatgptMode(codex_auth_path()));
    }

    let tokens = root.get("tokens").and_then(|v| v.as_object());

    let access_token = tokens
        .and_then(|t| t.get("access_token"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(CodexAuthError::MissingToken)?;

    let account_id =
        tokens.and_then(|t| t.get("account_id")).and_then(|v| v.as_str()).unwrap_or("").to_string();

    Ok(CodexCreds { access_token: access_token.to_string(), account_id })
}

/// Load credentials from the on-disk Codex auth file.
pub fn load_codex_creds() -> Result<CodexCreds, CodexAuthError> {
    let path = codex_auth_path();
    let contents = std::fs::read_to_string(&path).map_err(|_| CodexAuthError::NotFound(path))?;
    parse_codex_auth(&contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chatgpt_auth() -> &'static str {
        r#"{
            "auth_mode": "chatgpt",
            "OPENAI_API_KEY": null,
            "tokens": {
                "id_token": "header.payload.sig",
                "access_token": "access-abc",
                "refresh_token": "refresh-xyz",
                "account_id": "acct-123"
            },
            "last_refresh": "2026-06-14T00:00:00Z"
        }"#
    }

    #[test]
    fn parses_chatgpt_mode_credentials() {
        let creds = parse_codex_auth(chatgpt_auth()).unwrap();
        assert_eq!(creds.access_token, "access-abc");
        assert_eq!(creds.account_id, "acct-123");
    }

    #[test]
    fn rejects_apikey_mode() {
        let json = r#"{"auth_mode":"apikey","OPENAI_API_KEY":"sk-real","tokens":null}"#;
        let err = parse_codex_auth(json).unwrap_err();
        assert!(matches!(err, CodexAuthError::NotChatgptMode(_)));
    }

    #[test]
    fn rejects_missing_access_token() {
        let json = r#"{"auth_mode":"chatgpt","tokens":{"account_id":"acct-123"}}"#;
        let err = parse_codex_auth(json).unwrap_err();
        assert!(matches!(err, CodexAuthError::MissingToken));
    }

    #[test]
    fn rejects_empty_access_token() {
        let json =
            r#"{"auth_mode":"chatgpt","tokens":{"access_token":"","account_id":"acct-123"}}"#;
        let err = parse_codex_auth(json).unwrap_err();
        assert!(matches!(err, CodexAuthError::MissingToken));
    }

    #[test]
    fn rejects_malformed_json() {
        let err = parse_codex_auth("not json {").unwrap_err();
        assert!(matches!(err, CodexAuthError::Malformed(_)));
    }

    #[test]
    fn path_honors_codex_home() {
        // SAFETY: single-threaded test process for env mutation.
        unsafe { std::env::set_var("CODEX_HOME", "/tmp/skald-codex-test-home") };
        let p = codex_auth_path();
        unsafe { std::env::remove_var("CODEX_HOME") };
        assert_eq!(p, PathBuf::from("/tmp/skald-codex-test-home/auth.json"));
    }
}
