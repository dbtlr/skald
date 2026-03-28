use crate::error::{Result, SkaldError};
use std::collections::HashMap;
use tera::{Context, Tera};

#[derive(Debug, Clone)]
pub struct PromptContext {
    pub vars: HashMap<String, String>,
}

impl PromptContext {
    pub fn new() -> Self {
        Self { vars: HashMap::new() }
    }

    pub fn set(mut self, key: &str, value: &str) -> Self {
        self.vars.insert(key.to_string(), value.to_string());
        self
    }

    fn to_tera_context(&self) -> Context {
        let mut ctx = Context::new();
        for (k, v) in &self.vars {
            ctx.insert(k, v);
        }
        ctx
    }
}

impl Default for PromptContext {
    fn default() -> Self {
        Self::new()
    }
}

pub fn mock_prompt_context() -> PromptContext {
    PromptContext::new()
        .set("branch", "feature/example-branch")
        .set(
            "diff_stat",
            " src/main.rs | 10 +++++-----\n 2 files changed, 5 insertions(+), 5 deletions(-)",
        )
        .set("context", "")
        .set("language", "English")
        .set("num_suggestions", "3")
        .set("files_changed", "src/main.rs, src/lib.rs")
        .set("title", "feat(auth): add token refresh")
        .set("target_branch", "main")
        .set(
            "commit_log",
            "a1b2c3d feat(auth): add token refresh\nb2c3d4e fix(auth): handle expired tokens",
        )
}

pub fn render_prompt(template: &str, ctx: &PromptContext) -> Result<String> {
    let mut tera = Tera::default();
    tera.add_raw_template("prompt", template)
        .map_err(|e| SkaldError::PromptRender { name: "prompt".into(), detail: e.to_string() })?;
    tera.render("prompt", &ctx.to_tera_context())
        .map_err(|e| SkaldError::PromptRender { name: "prompt".into(), detail: e.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts::builtin;

    #[test]
    fn renders_simple_variables() {
        let ctx = mock_prompt_context();
        let result = render_prompt("Branch: {{ branch }}", &ctx).unwrap();
        assert_eq!(result, "Branch: feature/example-branch");
    }

    #[test]
    fn renders_conditional_blocks() {
        let ctx = PromptContext::new().set("context", "");
        let template = "{% if context %}Has context: {{ context }}{% endif %}Done";
        let result = render_prompt(template, &ctx).unwrap();
        assert_eq!(result, "Done");
    }

    #[test]
    fn renders_with_context_set() {
        let ctx = PromptContext::new().set("context", "some user context");
        let template = "{% if context %}Has context: {{ context }}{% endif %}";
        let result = render_prompt(template, &ctx).unwrap();
        assert_eq!(result, "Has context: some user context");
    }

    #[test]
    fn renders_language_conditional() {
        let ctx = PromptContext::new().set("language", "Spanish");
        let template = "{% if language != \"English\" %}Write in {{ language }}.{% endif %}";
        let result = render_prompt(template, &ctx).unwrap();
        assert_eq!(result, "Write in Spanish.");
    }

    #[test]
    fn renders_builtin_commit_title() {
        let ctx = mock_prompt_context();
        let result = render_prompt(builtin::COMMIT_TITLE, &ctx).unwrap();
        assert!(result.contains("3 commit messages"), "should contain num_suggestions");
        assert!(result.contains("src/main.rs | 10"), "should contain diff_stat content");
    }

    #[test]
    fn invalid_template_returns_error() {
        let ctx = PromptContext::new();
        let result = render_prompt("{{ undefined_var }}", &ctx);
        assert!(result.is_err());
        match result.unwrap_err() {
            SkaldError::PromptRender { name, detail } => {
                assert_eq!(name, "prompt");
                assert!(!detail.is_empty());
            }
            other => panic!("Expected PromptRender error, got: {other:?}"),
        }
    }
}
