use std::process::Command;
use std::time::{Duration, SystemTime};

use tracing::{debug, info};

use super::{Category, CheckResult};
use crate::config;

/// Shell out to a command with one flag, returning combined stdout/stderr on success.
fn check_command_available(cmd: &str, version_flag: &str) -> Option<String> {
    Command::new(cmd).arg(version_flag).output().ok().filter(|o| o.status.success()).map(|o| {
        let stdout = String::from_utf8_lossy(&o.stdout);
        let stderr = String::from_utf8_lossy(&o.stderr);
        let combined = format!("{}{}", stdout.trim(), stderr.trim());
        combined.trim().to_string()
    })
}

// ---------------------------------------------------------------------------
// Environment checks
// ---------------------------------------------------------------------------

pub fn environment_checks() -> Vec<CheckResult> {
    debug!("running environment checks");
    vec![check_git(), check_git_repo(), check_gh()]
}

fn check_git() -> CheckResult {
    debug!("checking git availability");
    match check_command_available("git", "--version") {
        Some(version) => {
            info!(version = %version, "git found");
            CheckResult::pass("git", &version).with_category(Category::Environment)
        }
        None => CheckResult::fail("git", "git is not installed or not in PATH")
            .with_category(Category::Environment)
            .with_suggestion("Install git: https://git-scm.com/downloads"),
    }
}

fn check_git_repo() -> CheckResult {
    let output = Command::new("git").args(["rev-parse", "--is-inside-work-tree"]).output();

    match output {
        Ok(o) if o.status.success() => CheckResult::pass("git_repo", "inside a git repository")
            .with_category(Category::Environment),
        _ => CheckResult::warn("git_repo", "not inside a git repository")
            .with_category(Category::Environment)
            .with_suggestion("Run `git init` or cd into a git repository"),
    }
}

fn check_gh() -> CheckResult {
    match check_command_available("gh", "--version") {
        Some(version) => {
            let first_line = version.lines().next().unwrap_or(&version).to_string();
            CheckResult::pass("gh", &first_line).with_category(Category::Environment)
        }
        None => CheckResult::warn("gh", "GitHub CLI is not installed")
            .with_category(Category::Environment)
            .with_suggestion("Install gh: https://cli.github.com/"),
    }
}

// ---------------------------------------------------------------------------
// Configuration checks
// ---------------------------------------------------------------------------

pub fn config_checks(fix: bool) -> Vec<CheckResult> {
    debug!(fix, "running configuration checks");
    vec![check_config_dir(fix), check_config_file(fix), check_project_config()]
}

fn check_config_dir(fix: bool) -> CheckResult {
    let dir = config::config_dir();
    debug!(path = %dir.display(), "checking config directory");

    if dir.is_dir() {
        return CheckResult::pass("config_dir", &format!("{}", dir.display()))
            .with_category(Category::Configuration);
    }

    if fix {
        match std::fs::create_dir_all(&dir) {
            Ok(()) => CheckResult::fixed("config_dir", &format!("created {}", dir.display()))
                .with_category(Category::Configuration)
                .with_was("missing"),
            Err(e) => CheckResult::fail("config_dir", &format!("failed to create: {e}"))
                .with_category(Category::Configuration),
        }
    } else {
        CheckResult::fail("config_dir", &format!("{} does not exist", dir.display()))
            .with_category(Category::Configuration)
            .with_suggestion(&format!("Run `sk doctor --fix` or `mkdir -p {}`", dir.display()))
    }
}

fn check_config_file(fix: bool) -> CheckResult {
    let path = config::global_config_path();

    if path.is_file() {
        match config::load_file(&path) {
            Ok(Some(_)) => CheckResult::pass("config_file", &format!("{}", path.display()))
                .with_category(Category::Configuration),
            Ok(None) => {
                // Shouldn't happen since is_file() passed, but handle gracefully
                CheckResult::warn("config_file", "config file disappeared during check")
                    .with_category(Category::Configuration)
            }
            Err(e) => CheckResult::fail("config_file", &format!("parse error: {e}"))
                .with_category(Category::Configuration)
                .with_suggestion("Fix the YAML syntax in your config file"),
        }
    } else if fix {
        let dir = config::config_dir();
        if !dir.is_dir() {
            return CheckResult::fail(
                "config_file",
                "config directory does not exist; fix config_dir first",
            )
            .with_category(Category::Configuration);
        }
        let default_config = "# Skald configuration\n# See: https://github.com/skald-cli/skald\n";
        match std::fs::write(&path, default_config) {
            Ok(()) => CheckResult::fixed("config_file", &format!("created {}", path.display()))
                .with_category(Category::Configuration)
                .with_was("missing"),
            Err(e) => CheckResult::fail("config_file", &format!("failed to create: {e}"))
                .with_category(Category::Configuration),
        }
    } else {
        CheckResult::warn("config_file", &format!("{} does not exist", path.display()))
            .with_category(Category::Configuration)
            .with_suggestion("Run `sk doctor --fix` to create a default config")
    }
}

fn check_project_config() -> CheckResult {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(_) => {
            return CheckResult::warn("project_config", "could not determine current directory")
                .with_category(Category::Configuration);
        }
    };

    match config::discover_project_config(&cwd) {
        Some(path) => match config::load_file(&path) {
            Ok(Some(_)) => CheckResult::pass("project_config", &format!("{}", path.display()))
                .with_category(Category::Configuration),
            Ok(None) => CheckResult::warn("project_config", "project config disappeared")
                .with_category(Category::Configuration),
            Err(e) => CheckResult::warn("project_config", &format!("parse error: {e}"))
                .with_category(Category::Configuration)
                .with_suggestion("Fix the YAML syntax in your .skaldrc.yaml"),
        },
        None => CheckResult::pass("project_config", "no .skaldrc.yaml (optional)")
            .with_category(Category::Configuration),
    }
}

// ---------------------------------------------------------------------------
// Provider checks
// ---------------------------------------------------------------------------

pub fn provider_checks(full: bool) -> Vec<CheckResult> {
    debug!(full, "running provider checks");
    let mut results = vec![check_claude_cli()];

    if full {
        let test_result = Command::new("claude")
            .args(["-p", "Reply with exactly: ok", "--max-turns", "1"])
            .output();

        match test_result {
            Ok(output) if output.status.success() => {
                let response = String::from_utf8_lossy(&output.stdout);
                if response.to_lowercase().contains("ok") {
                    results.push(
                        CheckResult::pass(
                            "claude_connectivity",
                            "Claude CLI responded successfully",
                        )
                        .with_category(Category::Provider),
                    );
                } else {
                    results.push(
                        CheckResult::warn(
                            "claude_connectivity",
                            "Claude CLI responded but output unexpected",
                        )
                        .with_category(Category::Provider)
                        .with_suggestion("Check Claude CLI authentication"),
                    );
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                results.push(
                    CheckResult::fail(
                        "claude_connectivity",
                        &format!(
                            "Claude CLI failed: {}",
                            stderr.lines().next().unwrap_or("unknown error")
                        ),
                    )
                    .with_category(Category::Provider)
                    .with_suggestion("Run `claude` to check authentication"),
                );
            }
            Err(e) => {
                results.push(
                    CheckResult::fail(
                        "claude_connectivity",
                        &format!("Could not run Claude CLI: {e}"),
                    )
                    .with_category(Category::Provider)
                    .with_suggestion("Install: npm install -g @anthropic-ai/claude-code"),
                );
            }
        }
    }

    results
}

fn check_claude_cli() -> CheckResult {
    match check_command_available("claude", "--version") {
        Some(version) => {
            CheckResult::pass("claude_cli", &version).with_category(Category::Provider)
        }
        None => CheckResult::fail("claude_cli", "Claude CLI is not installed or not in PATH")
            .with_category(Category::Provider)
            .with_suggestion("Install Claude CLI: https://docs.anthropic.com/en/docs/claude-cli"),
    }
}

// ---------------------------------------------------------------------------
// Maintenance checks
// ---------------------------------------------------------------------------

pub fn maintenance_checks(fix: bool) -> Vec<CheckResult> {
    debug!(fix, "running maintenance checks");
    vec![check_log_dir(fix), check_stale_logs(fix), check_version()]
}

fn check_log_dir(fix: bool) -> CheckResult {
    let dir = config::log_dir();

    if dir.is_dir() {
        return CheckResult::pass("log_dir", &format!("{}", dir.display()))
            .with_category(Category::Maintenance);
    }

    if fix {
        match std::fs::create_dir_all(&dir) {
            Ok(()) => CheckResult::fixed("log_dir", &format!("created {}", dir.display()))
                .with_category(Category::Maintenance)
                .with_was("missing"),
            Err(e) => CheckResult::fail("log_dir", &format!("failed to create: {e}"))
                .with_category(Category::Maintenance),
        }
    } else {
        CheckResult::warn("log_dir", &format!("{} does not exist", dir.display()))
            .with_category(Category::Maintenance)
            .with_suggestion(&format!("Run `sk doctor --fix` or `mkdir -p {}`", dir.display()))
    }
}

fn count_stale_logs(retention_days: u64) -> std::io::Result<usize> {
    let log_dir = config::log_dir();
    if !log_dir.exists() {
        return Ok(0);
    }

    let cutoff = SystemTime::now() - Duration::from_secs(retention_days * 86400);
    let mut count = 0;

    for entry in std::fs::read_dir(&log_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if let Ok(modified) = metadata.modified()
            && modified < cutoff
        {
            count += 1;
        }
    }

    Ok(count)
}

fn check_stale_logs(fix: bool) -> CheckResult {
    let stale = match count_stale_logs(14) {
        Ok(n) => n,
        Err(_) => {
            return CheckResult::pass("stale_logs", "log directory not accessible")
                .with_category(Category::Maintenance);
        }
    };

    if stale == 0 {
        return CheckResult::pass("stale_logs", "no stale log files")
            .with_category(Category::Maintenance);
    }

    if fix {
        match crate::logging::prune_old_logs(14) {
            Ok(pruned) => CheckResult::fixed(
                "stale_logs",
                &format!("pruned {pruned} log file(s) older than 14 days"),
            )
            .with_category(Category::Maintenance)
            .with_was(&format!("{stale} stale file(s)")),
            Err(e) => CheckResult::fail("stale_logs", &format!("failed to prune: {e}"))
                .with_category(Category::Maintenance),
        }
    } else {
        CheckResult::warn("stale_logs", &format!("{stale} log file(s) older than 14 days"))
            .with_category(Category::Maintenance)
            .with_suggestion("Run `sk doctor --fix` to prune old logs")
    }
}

fn check_version() -> CheckResult {
    debug!("checking for version updates");
    match crate::upgrade::check_latest_version() {
        Some(info) if info.update_available => {
            info!(current = %info.current, latest = %info.latest, "update available");
            CheckResult::warn(
                "version",
                &format!("Update available: v{} → v{}", info.current, info.latest),
            )
            .with_category(Category::Maintenance)
            .with_suggestion("Run `sk upgrade` to update")
        }
        Some(info) => CheckResult::pass("version", &format!("up to date (v{})", info.current))
            .with_category(Category::Maintenance),
        None => {
            debug!("version check failed — network unavailable");
            CheckResult::pass("version", "version check skipped (network unavailable)")
                .with_category(Category::Maintenance)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doctor::Category;
    use serial_test::serial;

    #[test]
    fn environment_checks_have_correct_category() {
        let results = environment_checks();
        for r in &results {
            assert_eq!(r.category, Category::Environment, "check '{}' has wrong category", r.name);
        }
    }

    #[test]
    fn git_check_passes() {
        // git should be available in any dev environment
        let result = check_git();
        assert_eq!(result.status, super::super::CheckStatus::Pass);
        assert!(result.detail.contains("git version"));
    }

    #[test]
    #[serial]
    fn config_checks_have_correct_category() {
        let tmp = tempfile::tempdir().unwrap();
        unsafe { std::env::set_var("XDG_CONFIG_HOME", tmp.path()) };
        let results = config_checks(false);
        for r in &results {
            assert_eq!(
                r.category,
                Category::Configuration,
                "check '{}' has wrong category",
                r.name
            );
        }
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
    }

    #[test]
    #[serial]
    fn config_fix_creates_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let config_home = tmp.path().join("unique_config_fix_test");
        unsafe { std::env::set_var("XDG_CONFIG_HOME", &config_home) };

        let dir = config::config_dir();
        assert!(!dir.exists());

        let result = check_config_dir(true);
        assert_eq!(result.status, super::super::CheckStatus::Fixed);
        assert!(dir.exists());

        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
    }

    #[test]
    #[serial]
    fn maintenance_checks_have_correct_category() {
        let tmp = tempfile::tempdir().unwrap();
        unsafe { std::env::set_var("XDG_CONFIG_HOME", tmp.path()) };
        let results = maintenance_checks(false);
        for r in &results {
            assert_eq!(r.category, Category::Maintenance, "check '{}' has wrong category", r.name);
        }
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
    }

    #[test]
    fn provider_checks_have_correct_category() {
        let results = provider_checks(false);
        for r in &results {
            assert_eq!(r.category, Category::Provider, "check '{}' has wrong category", r.name);
        }
    }
}
