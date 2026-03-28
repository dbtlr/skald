use std::path::{Path, PathBuf};

pub fn config_dir() -> PathBuf {
    // Respect XDG_CONFIG_HOME on all platforms (important for testing and Linux convention)
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("skald");
    }
    dirs::config_dir().map(|d| d.join("skald")).unwrap_or_else(|| PathBuf::from(".skald"))
}

pub fn log_dir() -> PathBuf {
    config_dir().join("logs")
}

pub fn global_config_path() -> PathBuf {
    config_dir().join("config.yaml")
}

/// Walk up from `start` looking for `.toolrc.yaml`, returning the first found.
pub fn discover_project_config(start: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        let candidate = dir.join(".toolrc.yaml");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}
