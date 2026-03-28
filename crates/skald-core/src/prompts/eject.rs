use crate::error::{Result, SkaldError};
use crate::prompts::builtin;
use std::path::Path;

pub fn eject_prompts(target_dir: &Path, names: Option<&[&str]>) -> Result<Vec<String>> {
    let template_names = match names {
        Some(names) => {
            for name in names {
                if builtin::get_builtin(name).is_none() {
                    return Err(SkaldError::PromptNotFound { name: name.to_string() });
                }
            }
            names.to_vec()
        }
        None => builtin::all_template_names(),
    };

    std::fs::create_dir_all(target_dir)
        .map_err(|e| SkaldError::PromptEject { name: "prompts".into(), detail: e.to_string() })?;

    let mut written = Vec::new();
    for name in template_names {
        let path = target_dir.join(format!("{name}.md"));
        if path.exists() {
            continue; // don't overwrite existing
        }
        let content = builtin::get_builtin(name).unwrap();
        let full_content = format!("{}{content}", builtin::EJECT_HEADER);
        std::fs::write(&path, full_content).map_err(|e| SkaldError::PromptEject {
            name: name.to_string(),
            detail: e.to_string(),
        })?;
        written.push(name.to_string());
    }

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eject_all_creates_five_files() {
        let dir = tempfile::tempdir().unwrap();
        let written = eject_prompts(dir.path(), None).unwrap();
        assert_eq!(written.len(), 5);
        for name in builtin::all_template_names() {
            let path = dir.path().join(format!("{name}.md"));
            assert!(path.exists(), "Expected {name}.md to exist");
        }
    }

    #[test]
    fn ejected_file_has_header_and_content() {
        let dir = tempfile::tempdir().unwrap();
        eject_prompts(dir.path(), None).unwrap();
        let content = std::fs::read_to_string(dir.path().join("commit-title.md")).unwrap();
        assert!(content.starts_with("{#"), "Should start with eject header comment");
        assert!(
            content.contains("{{ branch }}") || content.contains("{{ diff_stat }}"),
            "Should contain template variables"
        );
        assert!(content.contains("conventional commit"), "Should contain template content");
    }

    #[test]
    fn eject_single_template() {
        let dir = tempfile::tempdir().unwrap();
        let written = eject_prompts(dir.path(), Some(&["system"])).unwrap();
        assert_eq!(written.len(), 1);
        assert_eq!(written[0], "system");
        assert!(dir.path().join("system.md").exists());
        // Other templates should NOT exist
        assert!(!dir.path().join("commit-title.md").exists());
    }

    #[test]
    fn eject_unknown_template_errors() {
        let dir = tempfile::tempdir().unwrap();
        let result = eject_prompts(dir.path(), Some(&["nonexistent"]));
        assert!(result.is_err());
        match result.unwrap_err() {
            SkaldError::PromptNotFound { name } => {
                assert_eq!(name, "nonexistent");
            }
            other => panic!("Expected PromptNotFound, got: {other:?}"),
        }
    }

    #[test]
    fn eject_does_not_overwrite_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("system.md");
        std::fs::write(&path, "custom content").unwrap();

        let written = eject_prompts(dir.path(), Some(&["system"])).unwrap();
        assert!(written.is_empty(), "Should not have written anything");

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "custom content", "File should be preserved");
    }
}
