use std::path::Path;

use crate::core::config::paths::config_dir;
use crate::core::error::{Result, SkaldError};

use super::builtin::get_builtin;

/// Resolve a prompt template by name using the resolution chain:
/// CLI flag -> project `.skald/prompts/` -> global `~/.config/skald/prompts/` -> built-in.
pub fn resolve_template(
    name: &str,
    prompt_flag: Option<&Path>,
    project_dir: Option<&Path>,
) -> Result<String> {
    let global_prompts = config_dir().join("prompts");
    let project_prompts = project_dir.map(|d| d.join(".skald").join("prompts"));
    resolve_template_with_dirs(name, prompt_flag, project_prompts.as_deref(), Some(&global_prompts))
}

/// Testable version with injectable directory paths.
pub fn resolve_template_with_dirs(
    name: &str,
    prompt_flag: Option<&Path>,
    project_prompts_dir: Option<&Path>,
    global_prompts_dir: Option<&Path>,
) -> Result<String> {
    // 1. CLI flag — direct file path
    if let Some(flag_path) = prompt_flag {
        return std::fs::read_to_string(flag_path).map_err(|e| SkaldError::PromptNotFound {
            name: format!("{} ({}): {}", name, flag_path.display(), e),
        });
    }

    let filename = format!("{name}.md");

    // 2. Project-level override
    if let Some(project_dir) = project_prompts_dir {
        let candidate = project_dir.join(&filename);
        if candidate.is_file() {
            return std::fs::read_to_string(&candidate).map_err(|e| SkaldError::PromptRender {
                name: name.to_string(),
                detail: format!("Failed to read {}: {}", candidate.display(), e),
            });
        }
    }

    // 3. Global override
    if let Some(global_dir) = global_prompts_dir {
        let candidate = global_dir.join(&filename);
        if candidate.is_file() {
            return std::fs::read_to_string(&candidate).map_err(|e| SkaldError::PromptRender {
                name: name.to_string(),
                detail: format!("Failed to read {}: {}", candidate.display(), e),
            });
        }
    }

    // 4. Built-in fallback
    get_builtin(name)
        .map(String::from)
        .ok_or_else(|| SkaldError::PromptNotFound { name: name.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn setup_dir(base: &Path, name: &str, content: &str) -> PathBuf {
        let dir = base.join("prompts");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(format!("{name}.md")), content).unwrap();
        dir
    }

    #[test]
    fn builtin_fallback_when_no_files() {
        let result = resolve_template_with_dirs("system", None, None, None).unwrap();
        assert!(result.contains("senior software developer"));
    }

    #[test]
    fn global_overrides_builtin() {
        let tmp = tempfile::tempdir().unwrap();
        let global_dir = setup_dir(tmp.path(), "system", "global override");

        let result = resolve_template_with_dirs("system", None, None, Some(&global_dir)).unwrap();
        assert_eq!(result, "global override");
    }

    #[test]
    fn project_overrides_global() {
        let tmp = tempfile::tempdir().unwrap();
        let global_dir = setup_dir(&tmp.path().join("global"), "system", "global override");
        let project_dir = setup_dir(&tmp.path().join("project"), "system", "project override");

        let result =
            resolve_template_with_dirs("system", None, Some(&project_dir), Some(&global_dir))
                .unwrap();
        assert_eq!(result, "project override");
    }

    #[test]
    fn cli_flag_overrides_all() {
        let tmp = tempfile::tempdir().unwrap();
        let global_dir = setup_dir(&tmp.path().join("global"), "system", "global override");
        let project_dir = setup_dir(&tmp.path().join("project"), "system", "project override");
        let cli_file = tmp.path().join("custom.md");
        fs::write(&cli_file, "cli override").unwrap();

        let result = resolve_template_with_dirs(
            "system",
            Some(&cli_file),
            Some(&project_dir),
            Some(&global_dir),
        )
        .unwrap();
        assert_eq!(result, "cli override");
    }

    #[test]
    fn cli_flag_missing_file_errors() {
        let result = resolve_template_with_dirs(
            "system",
            Some(Path::new("/nonexistent/custom.md")),
            None,
            None,
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SkaldError::PromptNotFound { .. }));
    }

    #[test]
    fn unknown_template_errors() {
        let result = resolve_template_with_dirs("nonexistent", None, None, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SkaldError::PromptNotFound { .. }));
    }
}
