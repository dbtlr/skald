use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    dirs::config_dir().map(|d| d.join("skald")).unwrap_or_else(|| PathBuf::from(".skald"))
}

pub fn log_dir() -> PathBuf {
    config_dir().join("logs")
}
