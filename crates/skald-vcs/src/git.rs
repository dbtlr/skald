use std::path::PathBuf;
use std::process::Command;

use crate::diff_filter::filter_diff;
use crate::{DiffOptions, DiffResult, StageMode, VcsAdapter, VcsError};

pub struct GitAdapter;

impl GitAdapter {
    pub fn detect() -> Result<Self, VcsError> {
        let output = Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .output()
            .map_err(|e| VcsError::CommandFailed(format!("failed to run git: {e}")))?;

        if output.status.success() { Ok(Self) } else { Err(VcsError::NotInRepo) }
    }

    fn run_git(&self, args: &[&str]) -> Result<String, VcsError> {
        let output = Command::new("git")
            .args(args)
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

    fn commit(&self, message: &str) -> Result<String, VcsError> {
        self.run_git(&["commit", "-m", message])
    }

    fn commit_amend(&self, message: &str) -> Result<String, VcsError> {
        self.run_git(&["commit", "--amend", "-m", message])
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
