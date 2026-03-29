use std::collections::HashMap;

use crate::error::{Result, SkaldError};

pub const BUILTIN_COMMANDS: &[&str] = &["commit", "pr", "config", "alias", "doctor", "completions"];

/// Validate all aliases in the map.
///
/// Rules:
/// 1. Name must not shadow a built-in command
/// 2. First token of expansion must be a built-in command
/// 3. First token must not reference another alias (no recursion)
pub fn validate_aliases(aliases: &HashMap<String, String>) -> Result<()> {
    for (name, expansion) in aliases {
        if BUILTIN_COMMANDS.contains(&name.as_str()) {
            return Err(SkaldError::AliasShadowsBuiltin {
                name: name.clone(),
                command: name.clone(),
            });
        }

        let first_token = expansion.split_whitespace().next().unwrap_or("");

        if aliases.contains_key(first_token) {
            return Err(SkaldError::AliasRecursive { name: name.clone() });
        }

        if !BUILTIN_COMMANDS.contains(&first_token) {
            return Err(SkaldError::AliasInvalidCommand { name: name.clone() });
        }
    }

    Ok(())
}

/// Expand an alias if the subcommand position matches an alias key.
///
/// Skips over leading flags (arguments starting with `-`) to find the subcommand,
/// then expands it if it matches an alias. Flags before the alias are preserved.
///
/// Returns `Some(expanded_args)` if an alias matched, `None` otherwise.
pub fn expand_alias(args: &[String], aliases: &HashMap<String, String>) -> Option<Vec<String>> {
    // Find the first non-flag argument (the subcommand position).
    // We also need to skip flag values for flags that take a value (e.g. --format json).
    let mut cmd_idx = None;
    let mut skip_next = false;
    for (i, arg) in args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg.starts_with('-') {
            // Known global flags that consume the next argument
            if arg == "--format" {
                skip_next = true;
            }
            continue;
        }
        cmd_idx = Some(i);
        break;
    }

    let idx = cmd_idx?;
    let expansion = aliases.get(args[idx].as_str())?;

    let mut expanded: Vec<String> = args[..idx].to_vec();
    expanded.extend(expansion.split_whitespace().map(String::from));
    expanded.extend_from_slice(&args[idx + 1..]);
    Some(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(entries: &[(&str, &str)]) -> HashMap<String, String> {
        entries.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn simple_alias_expand() {
        let aliases = map(&[("ci", "commit -n 5")]);
        let result = expand_alias(&args(&["ci"]), &aliases).unwrap();
        assert_eq!(result, args(&["commit", "-n", "5"]));
    }

    #[test]
    fn user_args_appended() {
        let aliases = map(&[("ci", "commit -n 5")]);
        let result = expand_alias(&args(&["ci", "--no-extended"]), &aliases).unwrap();
        assert_eq!(result, args(&["commit", "-n", "5", "--no-extended"]));
    }

    #[test]
    fn non_alias_returns_none() {
        let aliases = map(&[("ci", "commit -n 5")]);
        assert!(expand_alias(&args(&["commit"]), &aliases).is_none());
    }

    #[test]
    fn empty_aliases_returns_none() {
        let aliases = HashMap::new();
        assert!(expand_alias(&args(&["anything"]), &aliases).is_none());
    }

    #[test]
    fn recursive_alias_error() {
        let aliases = map(&[("ci", "commit -n 5"), ("fast", "ci --quick")]);
        let err = validate_aliases(&aliases).unwrap_err();
        match err {
            SkaldError::AliasRecursive { name } => assert_eq!(name, "fast"),
            other => panic!("Expected AliasRecursive, got: {other}"),
        }
    }

    #[test]
    fn builtin_shadowing_error() {
        let aliases = map(&[("commit", "commit -n 5")]);
        let err = validate_aliases(&aliases).unwrap_err();
        match err {
            SkaldError::AliasShadowsBuiltin { name, command } => {
                assert_eq!(name, "commit");
                assert_eq!(command, "commit");
            }
            other => panic!("Expected AliasShadowsBuiltin, got: {other}"),
        }
    }

    #[test]
    fn invalid_command_error() {
        let aliases = map(&[("bad", "nonexistent --flag")]);
        let err = validate_aliases(&aliases).unwrap_err();
        match err {
            SkaldError::AliasInvalidCommand { name } => assert_eq!(name, "bad"),
            other => panic!("Expected AliasInvalidCommand, got: {other}"),
        }
    }

    #[test]
    fn alias_with_leading_flags() {
        let aliases = map(&[("ci", "commit -n 5")]);
        let result =
            expand_alias(&args(&["-v", "--quiet", "ci", "--no-extended"]), &aliases).unwrap();
        assert_eq!(result, args(&["-v", "--quiet", "commit", "-n", "5", "--no-extended"]));
    }

    #[test]
    fn alias_with_format_flag_skips_value() {
        let aliases = map(&[("ci", "commit -n 5")]);
        let result = expand_alias(&args(&["--format", "json", "ci"]), &aliases).unwrap();
        assert_eq!(result, args(&["--format", "json", "commit", "-n", "5"]));
    }

    #[test]
    fn no_subcommand_returns_none() {
        let aliases = map(&[("ci", "commit -n 5")]);
        assert!(expand_alias(&args(&["-v", "--quiet"]), &aliases).is_none());
    }
}
