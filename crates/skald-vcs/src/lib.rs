pub mod diff_filter;
pub mod git;

#[derive(Debug, Clone)]
pub struct DiffOptions {
    pub staged: bool,
    pub exclude_patterns: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DiffResult {
    pub diff: String,
    pub stat: String,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum StageMode {
    Tracked, // git add -u
    All,     // git add -A
}

#[derive(Debug, thiserror::Error)]
pub enum VcsError {
    #[error("Not in a repository")]
    NotInRepo,

    #[error("VCS command failed: {0}")]
    CommandFailed(String),

    #[error("{0}")]
    Other(String),
}

pub trait VcsAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn get_diff(&self, options: &DiffOptions) -> Result<DiffResult, VcsError>;
    fn commit(&self, message: &str) -> Result<String, VcsError>;
    fn commit_amend(&self, message: &str) -> Result<String, VcsError>;
    fn commit_with_body(&self, title: &str, body: &str) -> Result<String, VcsError>;
    fn commit_amend_with_body(&self, title: &str, body: &str) -> Result<String, VcsError>;
    fn get_current_branch(&self) -> Result<String, VcsError>;
    fn get_repo_root(&self) -> Result<std::path::PathBuf, VcsError>;
    fn has_staged_changes(&self) -> Result<bool, VcsError>;
    fn has_unstaged_changes(&self) -> Result<bool, VcsError>;
    fn stage(&self, mode: StageMode) -> Result<(), VcsError>;
}
