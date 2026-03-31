use std::path::Path;

use crate::config::aliases::validate_aliases;
use crate::config::loader::load_file;
use crate::error::{Result, SkaldError};

/// Add an alias to a YAML config file.
///
/// Creates the file if it doesn't exist. Fails if the alias already exists
/// unless `force` is true. Validates the full alias set before writing.
pub fn add_alias(name: &str, expansion: &str, path: &Path, force: bool) -> Result<()> {
    let mut config = load_file(path)?.unwrap_or_default();
    let aliases = config.aliases.get_or_insert_with(Default::default);

    if !force && let Some(existing) = aliases.get(name) {
        return Err(SkaldError::AliasAlreadyExists {
            name: name.to_string(),
            expansion: existing.clone(),
        });
    }

    aliases.insert(name.to_string(), expansion.to_string());
    validate_aliases(aliases)?;

    let yaml = serde_yaml_ng::to_string(&config)
        .map_err(|e| SkaldError::Other { message: format!("Failed to serialize config: {e}") })?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, yaml)?;
    Ok(())
}

/// Remove an alias from a YAML config file.
///
/// Fails if the file doesn't exist or the alias is not found.
/// `scope` is used in error messages (e.g. "global" or "project").
pub fn remove_alias(name: &str, path: &Path, scope: &str) -> Result<()> {
    let mut config = match load_file(path)? {
        Some(c) => c,
        None => {
            return Err(SkaldError::AliasNotFound {
                name: name.to_string(),
                scope: scope.to_string(),
            });
        }
    };

    let aliases = match config.aliases.as_mut() {
        Some(a) if a.contains_key(name) => a,
        _ => {
            return Err(SkaldError::AliasNotFound {
                name: name.to_string(),
                scope: scope.to_string(),
            });
        }
    };

    aliases.remove(name);

    let yaml = serde_yaml_ng::to_string(&config)
        .map_err(|e| SkaldError::Other { message: format!("Failed to serialize config: {e}") })?;
    std::fs::write(path, yaml)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_config(dir: &TempDir, content: &str) -> std::path::PathBuf {
        let path = dir.path().join("config.yaml");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn add_alias_to_empty_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.yaml");
        add_alias("ci", "commit -n 5", &path, false).unwrap();
        let config = load_file(&path).unwrap().unwrap();
        assert_eq!(config.aliases.unwrap()["ci"], "commit -n 5");
    }

    #[test]
    fn add_alias_to_existing_config() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "provider: claude\n");
        add_alias("ci", "commit -n 5", &path, false).unwrap();
        let config = load_file(&path).unwrap().unwrap();
        assert_eq!(config.provider.unwrap(), "claude");
        assert_eq!(config.aliases.unwrap()["ci"], "commit -n 5");
    }

    #[test]
    fn add_alias_preserves_existing_aliases() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "aliases:\n  ci: \"commit -n 5\"\n");
        add_alias("p", "pr", &path, false).unwrap();
        let config = load_file(&path).unwrap().unwrap();
        let aliases = config.aliases.unwrap();
        assert_eq!(aliases["ci"], "commit -n 5");
        assert_eq!(aliases["p"], "pr");
    }

    #[test]
    fn add_alias_fails_if_exists_without_force() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "aliases:\n  ci: \"commit -n 5\"\n");
        let err = add_alias("ci", "commit -n 10", &path, false).unwrap_err();
        assert!(matches!(err, SkaldError::AliasAlreadyExists { .. }));
    }

    #[test]
    fn add_alias_overwrites_with_force() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "aliases:\n  ci: \"commit -n 5\"\n");
        add_alias("ci", "commit -n 10", &path, true).unwrap();
        let config = load_file(&path).unwrap().unwrap();
        assert_eq!(config.aliases.unwrap()["ci"], "commit -n 10");
    }

    #[test]
    fn add_alias_validates_before_write() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.yaml");
        let err = add_alias("commit", "commit -n 5", &path, false).unwrap_err();
        assert!(matches!(err, SkaldError::AliasShadowsBuiltin { .. }));
        assert!(!path.exists());
    }

    #[test]
    fn add_alias_validates_expansion_targets_builtin() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.yaml");
        let err = add_alias("bad", "nonexistent --flag", &path, false).unwrap_err();
        assert!(matches!(err, SkaldError::AliasInvalidCommand { .. }));
    }

    #[test]
    fn remove_alias_succeeds() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "aliases:\n  ci: \"commit -n 5\"\n  p: \"pr\"\n");
        remove_alias("ci", &path, "global").unwrap();
        let config = load_file(&path).unwrap().unwrap();
        let aliases = config.aliases.unwrap();
        assert!(!aliases.contains_key("ci"));
        assert_eq!(aliases["p"], "pr");
    }

    #[test]
    fn remove_alias_not_found() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "aliases:\n  ci: \"commit -n 5\"\n");
        let err = remove_alias("nonexistent", &path, "global").unwrap_err();
        assert!(matches!(err, SkaldError::AliasNotFound { .. }));
    }

    #[test]
    fn remove_alias_no_config_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.yaml");
        let err = remove_alias("ci", &path, "global").unwrap_err();
        assert!(matches!(err, SkaldError::AliasNotFound { .. }));
    }
}
