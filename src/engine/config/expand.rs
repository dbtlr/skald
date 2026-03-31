use crate::engine::error::{Result, SkaldError};

use super::schema::RawConfig;

/// Expand environment variables in a string value.
///
/// Scans for `$` followed by `[A-Za-z_][A-Za-z0-9_]*` and replaces with
/// the environment variable value. Returns an error if the variable is not set.
pub fn expand_env_vars(value: &str, context: &str) -> Result<String> {
    let mut result = String::with_capacity(value.len());
    let chars: Vec<char> = value.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '$' && i + 1 < chars.len() && is_var_start(chars[i + 1]) {
            i += 1; // skip $
            let start = i;
            while i < chars.len() && is_var_char(chars[i]) {
                i += 1;
            }
            let name: String = chars[start..i].iter().collect();
            let val = std::env::var(&name).map_err(|_| SkaldError::EnvVarNotSet {
                name: name.clone(),
                context: context.to_string(),
            })?;
            result.push_str(&val);
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    Ok(result)
}

fn is_var_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_var_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Expand environment variables in all string fields of a RawConfig.
pub fn expand_config(config: &mut RawConfig) -> Result<()> {
    if let Some(ref v) = config.provider {
        config.provider = Some(expand_env_vars(v, "provider")?);
    }
    if let Some(ref v) = config.language {
        config.language = Some(expand_env_vars(v, "language")?);
    }
    if let Some(ref v) = config.pr_target {
        config.pr_target = Some(expand_env_vars(v, "pr_target")?);
    }
    if let Some(ref v) = config.platform {
        config.platform = Some(expand_env_vars(v, "platform")?);
    }
    if let Some(ref v) = config.vcs {
        config.vcs = Some(expand_env_vars(v, "vcs")?);
    }
    if let Some(ref providers) = config.providers {
        let mut expanded = providers.clone();
        for (name, pc) in expanded.iter_mut() {
            if let Some(ref v) = pc.model {
                pc.model = Some(expand_env_vars(v, &format!("providers.{name}.model"))?);
            }
            if let Some(ref v) = pc.api_key {
                pc.api_key = Some(expand_env_vars(v, &format!("providers.{name}.api_key"))?);
            }
            if let Some(ref v) = pc.base_url {
                pc.base_url = Some(expand_env_vars(v, &format!("providers.{name}.base_url"))?);
            }
        }
        config.providers = Some(expanded);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // SAFETY: These tests mutate env vars which is unsafe in Rust 2024.
    // Tests using set_var/remove_var are run with --test-threads=1 or accept
    // the inherent unsafety of env mutation in tests.

    #[test]
    fn env_var_set_expands_correctly() {
        unsafe { std::env::set_var("SK_TEST_VAR", "hello") };
        let result = expand_env_vars("$SK_TEST_VAR", "test").unwrap();
        assert_eq!(result, "hello");
        unsafe { std::env::remove_var("SK_TEST_VAR") };
    }

    #[test]
    fn env_var_not_set_returns_error() {
        unsafe { std::env::remove_var("SK_MISSING_VAR") };
        let err = expand_env_vars("$SK_MISSING_VAR", "test").unwrap_err();
        match err {
            SkaldError::EnvVarNotSet { name, .. } => assert_eq!(name, "SK_MISSING_VAR"),
            other => panic!("Expected EnvVarNotSet, got: {other}"),
        }
    }

    #[test]
    fn no_dollar_sign_returns_unchanged() {
        let result = expand_env_vars("no variables here", "test").unwrap();
        assert_eq!(result, "no variables here");
    }

    #[test]
    fn multiple_vars_expand() {
        unsafe { std::env::set_var("SK_A", "foo") };
        unsafe { std::env::set_var("SK_B", "bar") };
        let result = expand_env_vars("$SK_A-$SK_B", "test").unwrap();
        assert_eq!(result, "foo-bar");
        unsafe { std::env::remove_var("SK_A") };
        unsafe { std::env::remove_var("SK_B") };
    }

    #[test]
    fn partial_expansion_with_prefix_suffix() {
        unsafe { std::env::set_var("SK_MID", "middle") };
        let result = expand_env_vars("prefix-$SK_MID-suffix", "test").unwrap();
        assert_eq!(result, "prefix-middle-suffix");
        unsafe { std::env::remove_var("SK_MID") };
    }
}
