use std::io;
use std::io::Write;
use std::process::Command;

use tempfile::Builder;

/// Open `content` in the user's preferred editor and return the edited text.
///
/// Returns `Ok(Some(edited))` if the editor exits successfully with non-empty content.
/// Returns `Ok(None)` if the editor exits with an error or the content is empty.
/// Returns `Err` for I/O failures (temp file creation, etc.).
///
/// Editor resolution: `$VISUAL` → `$EDITOR` → `vi`.
pub fn edit_in_editor(content: &str, suffix: &str) -> io::Result<Option<String>> {
    let mut tmp = Builder::new()
        .prefix("skald-edit-")
        .suffix(suffix)
        .tempfile()?;

    tmp.write_all(content.as_bytes())?;
    tmp.flush()?;

    let path = tmp.path().to_path_buf();
    let editor = resolve_editor();

    // Split editor command in case it has args (e.g., "code --wait")
    let mut parts = editor.split_whitespace();
    let cmd = parts.next().unwrap_or("vi");
    let args: Vec<&str> = parts.collect();

    let status = Command::new(cmd)
        .args(&args)
        .arg(&path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !status.success() {
        return Ok(None);
    }

    let edited = std::fs::read_to_string(&path)?;
    let trimmed = edited.trim_end().to_string();

    if trimmed.is_empty() {
        return Ok(None);
    }

    Ok(Some(trimmed))
}

/// Resolve the editor command from environment variables.
/// Priority: $VISUAL → $EDITOR → vi
fn resolve_editor() -> String {
    if let Ok(visual) = std::env::var("VISUAL") {
        if !visual.is_empty() {
            return visual;
        }
    }
    if let Ok(editor) = std::env::var("EDITOR") {
        if !editor.is_empty() {
            return editor;
        }
    }
    "vi".to_string()
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    #[test]
    #[serial]
    fn resolve_editor_visual_takes_priority() {
        unsafe { std::env::set_var("VISUAL", "custom-visual"); }
        unsafe { std::env::set_var("EDITOR", "custom-editor"); }
        let editor = resolve_editor();
        assert_eq!(editor, "custom-visual");
        unsafe { std::env::remove_var("VISUAL"); }
        unsafe { std::env::remove_var("EDITOR"); }
    }

    #[test]
    #[serial]
    fn resolve_editor_falls_back_to_editor() {
        unsafe { std::env::remove_var("VISUAL"); }
        unsafe { std::env::set_var("EDITOR", "custom-editor"); }
        let editor = resolve_editor();
        assert_eq!(editor, "custom-editor");
        unsafe { std::env::remove_var("EDITOR"); }
    }

    #[test]
    #[serial]
    fn resolve_editor_falls_back_to_vi() {
        unsafe { std::env::remove_var("VISUAL"); }
        unsafe { std::env::remove_var("EDITOR"); }
        let editor = resolve_editor();
        assert_eq!(editor, "vi");
    }

    #[test]
    #[serial]
    fn edit_in_editor_with_true_returns_content() {
        // `true` exits 0 without modifying the file — content stays the same
        unsafe { std::env::set_var("VISUAL", "true"); }
        let result = edit_in_editor("hello world", ".md").unwrap();
        assert_eq!(result, Some("hello world".to_string()));
        unsafe { std::env::remove_var("VISUAL"); }
    }
}
