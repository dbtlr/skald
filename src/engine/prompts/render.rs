use crate::engine::error::{Result, SkaldError};
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

#[cfg(test)]
pub fn mock_prompt_context() -> PromptContext {
    PromptContext::new()
        .set("branch", "feature/add-oauth")
        .set("target_branch", "main")
        .set("diff_stat", " src/auth.rs | 45 +++++++++++++++++\n src/config.rs | 12 +++--\n 2 files changed, 50 insertions(+), 7 deletions(-)")
        .set("context", "")
        .set("language", "English")
        .set("num_suggestions", "3")
        .set("files_changed", "src/auth.rs, src/config.rs")
        .set("title", "feat(auth): add OAuth2 token refresh")
        .set("commit_log", "abc1234 feat(auth): add OAuth2 token refresh\ndef5678 fix(config): handle missing redirect URL")
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
    use crate::engine::prompts::builtin;

    #[test]
    fn renders_simple_variables() {
        let ctx = mock_prompt_context();
        let result = render_prompt("Branch: {{ branch }}", &ctx).unwrap();
        assert_eq!(result, "Branch: feature/add-oauth");
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
        assert!(result.contains("src/auth.rs | 45"), "should contain diff_stat content");
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
