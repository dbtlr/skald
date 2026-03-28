use crate::error::Result;

/// Context passed to prompt templates during rendering.
pub struct PromptContext {
    pub diff_stat: String,
    pub context: Option<String>,
    pub language: String,
    pub num_suggestions: usize,
    pub branch: Option<String>,
    pub target_branch: Option<String>,
    pub commit_log: Option<String>,
    pub title: Option<String>,
    pub files_changed: Vec<String>,
}

/// Render a named template with the given context.
pub fn render_prompt(_name: &str, _template: &str, _ctx: &PromptContext) -> Result<String> {
    todo!()
}

/// Create a mock context useful for testing template rendering.
pub fn mock_prompt_context() -> PromptContext {
    todo!()
}
