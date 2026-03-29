use std::path::PathBuf;
use std::process::Command;

use crate::diff_filter::filter_diff;
use crate::{DiffOptions, DiffResult, StageMode, VcsAdapter, VcsError};

pub struct GitAdapter {
    root: Option<PathBuf>,
}

impl GitAdapter {
    pub fn detect() -> Result<Self, VcsError> {
        let output = Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .output()
            .map_err(|e| VcsError::CommandFailed(format!("failed to run git: {e}")))?;

        if output.status.success() { Ok(Self { root: None }) } else { Err(VcsError::NotInRepo) }
    }

    fn run_git(&self, args: &[&str]) -> Result<String, VcsError> {
        let mut cmd = Command::new("git");
        cmd.args(args);
        if let Some(root) = &self.root {
            cmd.current_dir(root);
        }
        let output = cmd
            .output()
            .map_err(|e| VcsError::CommandFailed(format!("failed to run git: {e}")))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(VcsError::CommandFailed(stderr))
        }
    }
}

/// Parse the summary line from `git diff --stat` output.
///
/// Expected format: " N files changed, N insertions(+), N deletions(-)"
/// Returns (files_changed, insertions, deletions).
pub fn parse_stat_summary(stat: &str) -> (usize, usize, usize) {
    let last_line = match stat.lines().last() {
        Some(line) => line.trim(),
        None => return (0, 0, 0),
    };

    if last_line.is_empty() {
        return (0, 0, 0);
    }

    let mut files = 0;
    let mut insertions = 0;
    let mut deletions = 0;

    for part in last_line.split(',') {
        let part = part.trim();
        if part.contains("changed") {
            if let Some(n) = part.split_whitespace().next().and_then(|s| s.parse().ok()) {
                files = n;
            }
        } else if part.contains("insertion") {
            if let Some(n) = part.split_whitespace().next().and_then(|s| s.parse().ok()) {
                insertions = n;
            }
        } else if part.contains("deletion")
            && let Some(n) = part.split_whitespace().next().and_then(|s| s.parse().ok())
        {
            deletions = n;
        }
    }

    (files, insertions, deletions)
}

impl VcsAdapter for GitAdapter {
    fn name(&self) -> &str {
        "git"
    }

    fn get_diff(&self, options: &DiffOptions) -> Result<DiffResult, VcsError> {
        let mut diff_args = vec!["diff"];
        if options.staged {
            diff_args.push("--cached");
        }
        let raw_diff = self.run_git(&diff_args)?;

        let diff = filter_diff(&raw_diff, &options.exclude_patterns, true);

        let mut stat_args = vec!["diff", "--stat"];
        if options.staged {
            stat_args.push("--cached");
        }
        let stat = self.run_git(&stat_args)?;

        let (files_changed, insertions, deletions) = parse_stat_summary(&stat);

        Ok(DiffResult { diff, stat, files_changed, insertions, deletions })
    }

    fn get_branch_diff(&self, target: &str, options: &DiffOptions) -> Result<DiffResult, VcsError> {
        let range = format!("{target}...HEAD");
        let raw_diff = self.run_git(&["diff", &range])?;
        let diff = filter_diff(&raw_diff, &options.exclude_patterns, true);

        let stat = self.run_git(&["diff", "--stat", &range])?;
        let (files_changed, insertions, deletions) = parse_stat_summary(&stat);

        Ok(DiffResult { diff, stat, files_changed, insertions, deletions })
    }

    fn get_commit_log(&self, target: &str) -> Result<String, VcsError> {
        let range = format!("{target}..HEAD");
        self.run_git(&["log", &range, "--oneline"])
    }

    fn has_unpushed_commits(&self) -> Result<bool, VcsError> {
        let head = self.run_git(&["rev-parse", "HEAD"])?;
        match self.run_git(&["rev-parse", "@{u}"]) {
            Ok(upstream) => Ok(head != upstream),
            Err(_) => Ok(true), // no upstream configured
        }
    }

    fn get_remote_url(&self) -> Result<String, VcsError> {
        self.run_git(&["remote", "get-url", "origin"])
    }

    fn commit(&self, message: &str) -> Result<String, VcsError> {
        self.run_git(&["commit", "-m", message])
    }

    fn commit_amend(&self, message: &str) -> Result<String, VcsError> {
        self.run_git(&["commit", "--amend", "-m", message])
    }

    fn commit_with_body(&self, title: &str, body: &str) -> Result<String, VcsError> {
        self.run_git(&["commit", "-m", title, "-m", body])
    }

    fn commit_amend_with_body(&self, title: &str, body: &str) -> Result<String, VcsError> {
        self.run_git(&["commit", "--amend", "-m", title, "-m", body])
    }

    fn get_current_branch(&self) -> Result<String, VcsError> {
        self.run_git(&["branch", "--show-current"])
    }

    fn get_repo_root(&self) -> Result<PathBuf, VcsError> {
        let root = self.run_git(&["rev-parse", "--show-toplevel"])?;
        Ok(PathBuf::from(root))
    }

    fn has_staged_changes(&self) -> Result<bool, VcsError> {
        let output = Command::new("git")
            .args(["diff", "--cached", "--quiet"])
            .output()
            .map_err(|e| VcsError::CommandFailed(format!("failed to run git: {e}")))?;

        // exit 1 means there ARE staged changes
        Ok(!output.status.success())
    }

    fn has_unstaged_changes(&self) -> Result<bool, VcsError> {
        let output = Command::new("git")
            .args(["diff", "--quiet"])
            .output()
            .map_err(|e| VcsError::CommandFailed(format!("failed to run git: {e}")))?;

        // exit 1 means there ARE unstaged changes
        Ok(!output.status.success())
    }

    fn stage(&self, mode: StageMode) -> Result<(), VcsError> {
        let flag = match mode {
            StageMode::Tracked => "-u",
            StageMode::All => "-A",
        };
        self.run_git(&["add", flag])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command as Cmd;

    /// Create a minimal git repo in a temp dir and return (TempDir, GitAdapter).
    /// The repo has an initial commit on `main`.
    fn make_repo() -> (tempfile::TempDir, GitAdapter) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path();

        let run = |args: &[&str]| {
            let status = Cmd::new("git")
                .args(args)
                .current_dir(path)
                .status()
                .expect("git");
            assert!(status.success(), "git {args:?} failed");
        };

        run(&["init", "-b", "main"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test"]);

        fs::write(path.join("file.txt"), "hello\n").unwrap();
        run(&["add", "."]);
        run(&["commit", "-m", "initial commit"]);

        let adapter = GitAdapter { root: Some(path.to_path_buf()) };
        (dir, adapter)
    }

    #[test]
    fn get_commit_log_returns_commits_since_base() {
        let (dir, adapter) = make_repo();
        let path = dir.path();

        // Create a feature branch off main
        let run = |args: &[&str]| {
            Cmd::new("git").args(args).current_dir(path).status().expect("git").success()
        };

        run(&["checkout", "-b", "feature"]);

        fs::write(path.join("feature.txt"), "feature\n").unwrap();
        Cmd::new("git")
            .args(["add", "."])
            .current_dir(path)
            .status()
            .unwrap();
        Cmd::new("git")
            .args(["commit", "-m", "add feature"])
            .current_dir(path)
            .status()
            .unwrap();

        let log = adapter.get_commit_log("main").unwrap();
        assert!(log.contains("add feature"), "log should contain feature commit: {log}");
        assert!(!log.contains("initial commit"), "log should not contain base commit: {log}");
    }

    #[test]
    fn get_branch_diff_returns_diff_since_base() {
        let (dir, adapter) = make_repo();
        let path = dir.path();

        Cmd::new("git")
            .args(["checkout", "-b", "feature"])
            .current_dir(path)
            .status()
            .unwrap();

        fs::write(path.join("new.rs"), "fn foo() {}\n").unwrap();
        Cmd::new("git").args(["add", "."]).current_dir(path).status().unwrap();
        Cmd::new("git")
            .args(["commit", "-m", "add new.rs"])
            .current_dir(path)
            .status()
            .unwrap();

        let options = DiffOptions { staged: false, exclude_patterns: vec![] };
        let result = adapter.get_branch_diff("main", &options).unwrap();
        assert!(result.diff.contains("new.rs"), "diff should contain new.rs: {}", result.diff);
        assert_eq!(result.files_changed, 1);
        assert!(result.insertions > 0);
    }

    #[test]
    fn get_branch_diff_excludes_lock_files() {
        let (dir, adapter) = make_repo();
        let path = dir.path();

        Cmd::new("git")
            .args(["checkout", "-b", "feature"])
            .current_dir(path)
            .status()
            .unwrap();

        fs::write(path.join("Cargo.lock"), "[dependencies]\n").unwrap();
        fs::write(path.join("src.rs"), "fn bar() {}\n").unwrap();
        Cmd::new("git").args(["add", "."]).current_dir(path).status().unwrap();
        Cmd::new("git")
            .args(["commit", "-m", "add lock and src"])
            .current_dir(path)
            .status()
            .unwrap();

        let options = DiffOptions { staged: false, exclude_patterns: vec![] };
        let result = adapter.get_branch_diff("main", &options).unwrap();
        assert!(!result.diff.contains("Cargo.lock"), "lock file should be filtered");
        assert!(result.diff.contains("src.rs"), "src.rs should be in diff");
    }

    #[test]
    fn has_unpushed_commits_no_upstream_returns_true() {
        let (_dir, adapter) = make_repo();
        // No upstream configured, so should return true
        let result = adapter.has_unpushed_commits().unwrap();
        assert!(result, "should report unpushed when no upstream");
    }

    #[test]
    fn get_remote_url_no_remote_returns_error() {
        let (_dir, adapter) = make_repo();
        // No remote configured
        let result = adapter.get_remote_url();
        assert!(result.is_err(), "should error when no remote configured");
    }

    #[test]
    fn get_remote_url_returns_url() {
        let (dir, adapter) = make_repo();
        let path = dir.path();

        Cmd::new("git")
            .args(["remote", "add", "origin", "https://github.com/example/repo.git"])
            .current_dir(path)
            .status()
            .unwrap();

        let url = adapter.get_remote_url().unwrap();
        assert_eq!(url, "https://github.com/example/repo.git");
    }

    #[test]
    fn parse_stat_summary_full() {
        let stat = " src/main.rs | 10 ++++------\n src/lib.rs  |  5 +++--\n 2 files changed, 7 insertions(+), 8 deletions(-)";
        assert_eq!(parse_stat_summary(stat), (2, 7, 8));
    }

    #[test]
    fn parse_stat_summary_empty() {
        assert_eq!(parse_stat_summary(""), (0, 0, 0));
    }

    #[test]
    fn parse_stat_summary_multiple_files() {
        let stat = " 15 files changed, 200 insertions(+), 50 deletions(-)";
        assert_eq!(parse_stat_summary(stat), (15, 200, 50));
    }

    #[test]
    fn parse_stat_summary_insertions_only() {
        let stat = " 1 file changed, 3 insertions(+)";
        assert_eq!(parse_stat_summary(stat), (1, 3, 0));
    }

    #[test]
    fn parse_stat_summary_deletions_only() {
        let stat = " 1 file changed, 5 deletions(-)";
        assert_eq!(parse_stat_summary(stat), (1, 0, 5));
    }

    #[test]
    fn detect_in_git_repo() {
        // We're running inside the skald repo, so this should succeed
        let adapter = GitAdapter::detect();
        assert!(adapter.is_ok());
    }
}
