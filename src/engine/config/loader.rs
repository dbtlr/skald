use std::collections::HashMap;
use std::path::Path;

use crate::engine::error::{Result, SkaldError};

use super::aliases::validate_aliases;
use super::expand::expand_config;
use super::paths::{discover_project_config, global_config_path};
use super::schema::{ConfigSource, RawConfig, ResolvedConfig};

/// Load, merge, expand, validate, and resolve the full configuration.
pub fn load_config() -> Result<ResolvedConfig> {
    let global_path = global_config_path();
    let global = load_file(&global_path)?.unwrap_or_default();

    let cwd = std::env::current_dir()?;
    let project = discover_project_config(&cwd).map(|p| load_file(&p)).transpose()?.flatten();

    let mut merged = merge(&global, &project);
    expand_config(&mut merged)?;

    if let Some(ref aliases) = merged.aliases {
        validate_aliases(aliases)?;
    }

    Ok(resolve(&merged, &global, &project))
}

/// Load a YAML config file. Returns `Ok(None)` if the file doesn't exist.
pub fn load_file(path: &Path) -> Result<Option<RawConfig>> {
    match std::fs::read_to_string(path) {
        Ok(contents) => {
            let config: RawConfig =
                serde_yaml_ng::from_str(&contents).map_err(|e| SkaldError::ConfigParse {
                    path: path.display().to_string(),
                    line: e.location().map(|l| l.line()).unwrap_or(0),
                    detail: e.to_string(),
                })?;
            Ok(Some(config))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Merge global and project configs. Project values override global for scalar fields.
/// For aliases, project aliases replace same-name global aliases but other global aliases
/// are preserved.
pub fn merge(global: &RawConfig, project: &Option<RawConfig>) -> RawConfig {
    let project = match project {
        Some(p) => p,
        None => return global.clone(),
    };

    let aliases = match (&global.aliases, &project.aliases) {
        (Some(g), Some(p)) => {
            let mut merged = g.clone();
            merged.extend(p.clone());
            Some(merged)
        }
        (None, Some(p)) => Some(p.clone()),
        (Some(g), None) => Some(g.clone()),
        (None, None) => None,
    };

    let providers = match (&global.providers, &project.providers) {
        (Some(g), Some(p)) => {
            let mut merged = g.clone();
            merged.extend(p.clone());
            Some(merged)
        }
        (None, Some(p)) => Some(p.clone()),
        (Some(g), None) => Some(g.clone()),
        (None, None) => None,
    };

    RawConfig {
        provider: project.provider.clone().or(global.provider.clone()),
        language: project.language.clone().or(global.language.clone()),
        pr_target: project.pr_target.clone().or(global.pr_target.clone()),
        platform: project.platform.clone().or(global.platform.clone()),
        vcs: project.vcs.clone().or(global.vcs.clone()),
        providers,
        aliases,
    }
}

/// Fill defaults and build a ResolvedConfig with source tracking.
fn resolve(merged: &RawConfig, global: &RawConfig, project: &Option<RawConfig>) -> ResolvedConfig {
    let mut sources = HashMap::new();

    let provider = resolve_field(
        "provider",
        &merged.provider,
        &global.provider,
        project.as_ref().and_then(|p| p.provider.as_ref()),
        "claude",
        &mut sources,
    );
    let language = resolve_field(
        "language",
        &merged.language,
        &global.language,
        project.as_ref().and_then(|p| p.language.as_ref()),
        "English",
        &mut sources,
    );
    let pr_target = resolve_field(
        "pr_target",
        &merged.pr_target,
        &global.pr_target,
        project.as_ref().and_then(|p| p.pr_target.as_ref()),
        "main",
        &mut sources,
    );
    let platform = resolve_field(
        "platform",
        &merged.platform,
        &global.platform,
        project.as_ref().and_then(|p| p.platform.as_ref()),
        "github",
        &mut sources,
    );
    let vcs = resolve_field(
        "vcs",
        &merged.vcs,
        &global.vcs,
        project.as_ref().and_then(|p| p.vcs.as_ref()),
        "git",
        &mut sources,
    );

    ResolvedConfig {
        provider,
        language,
        pr_target,
        platform,
        vcs,
        providers: merged.providers.clone().unwrap_or_default(),
        aliases: merged.aliases.clone().unwrap_or_default(),
        sources,
    }
}

fn resolve_field(
    key: &str,
    merged_val: &Option<String>,
    global_val: &Option<String>,
    project_val: Option<&String>,
    default: &str,
    sources: &mut HashMap<String, ConfigSource>,
) -> String {
    match merged_val {
        Some(v) => {
            let source = if project_val.is_some() {
                ConfigSource::Project
            } else if global_val.is_some() {
                ConfigSource::Global
            } else {
                ConfigSource::Default
            };
            sources.insert(key.to_string(), source);
            v.clone()
        }
        None => {
            sources.insert(key.to_string(), ConfigSource::Default);
            default.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_yaml(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn valid_yaml_parses() {
        let f = write_yaml("provider: openai\nlanguage: Spanish\n");
        let config = load_file(f.path()).unwrap().unwrap();
        assert_eq!(config.provider.unwrap(), "openai");
        assert_eq!(config.language.unwrap(), "Spanish");
    }

    #[test]
    fn missing_file_returns_none() {
        let result = load_file(Path::new("/nonexistent/config.yaml")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn malformed_yaml_returns_parse_error() {
        let f = write_yaml("provider: [invalid yaml\n");
        let err = load_file(f.path()).unwrap_err();
        assert!(matches!(err, SkaldError::ConfigParse { .. }));
    }

    #[test]
    fn merge_project_overrides_global() {
        let global = RawConfig { provider: Some("openai".into()), ..Default::default() };
        let project = Some(RawConfig { provider: Some("claude-cli".into()), ..Default::default() });
        let merged = merge(&global, &project);
        assert_eq!(merged.provider.unwrap(), "claude-cli");
    }

    #[test]
    fn merge_global_preserved_when_project_omits() {
        let global = RawConfig { language: Some("Spanish".into()), ..Default::default() };
        let project = Some(RawConfig::default());
        let merged = merge(&global, &project);
        assert_eq!(merged.language.unwrap(), "Spanish");
    }

    #[test]
    fn merge_project_alias_replaces_same_name() {
        let global = RawConfig {
            aliases: Some([("ci".into(), "commit -n 5".into())].into()),
            ..Default::default()
        };
        let project = Some(RawConfig {
            aliases: Some([("ci".into(), "commit -n 10".into())].into()),
            ..Default::default()
        });
        let merged = merge(&global, &project);
        assert_eq!(merged.aliases.unwrap()["ci"], "commit -n 10");
    }

    #[test]
    fn merge_global_aliases_preserved() {
        let global = RawConfig {
            aliases: Some([("ci".into(), "commit -n 5".into()), ("p".into(), "pr".into())].into()),
            ..Default::default()
        };
        let project = Some(RawConfig {
            aliases: Some([("ci".into(), "commit -n 10".into())].into()),
            ..Default::default()
        });
        let merged = merge(&global, &project);
        let aliases = merged.aliases.unwrap();
        assert_eq!(aliases["ci"], "commit -n 10");
        assert_eq!(aliases["p"], "pr");
    }

    #[test]
    fn resolve_fills_defaults() {
        let merged = RawConfig::default();
        let global = RawConfig::default();
        let resolved = resolve(&merged, &global, &None);
        assert_eq!(resolved.provider, "claude");
        assert_eq!(resolved.language, "English");
        assert_eq!(resolved.pr_target, "main");
        assert_eq!(resolved.platform, "github");
        assert_eq!(resolved.vcs, "git");
    }

    #[test]
    fn resolve_tracks_sources() {
        let global = RawConfig { provider: Some("openai".into()), ..Default::default() };
        let project = Some(RawConfig { language: Some("Spanish".into()), ..Default::default() });
        let merged = merge(&global, &project);
        let resolved = resolve(&merged, &global, &project);

        assert_eq!(resolved.sources["provider"], ConfigSource::Global);
        assert_eq!(resolved.sources["language"], ConfigSource::Project);
        assert_eq!(resolved.sources["pr_target"], ConfigSource::Default);
    }
}
