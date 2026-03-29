use std::path::PathBuf;

use skald_core::config::schema::ResolvedConfig;
use skald_core::output::OutputFormat;
use skald_core::prompts::{PromptContext, render_prompt, resolve_template};
use skald_providers::claude_cli::ClaudeCliProvider;
use skald_providers::{CommitContext, Provider};
use skald_vcs::git::GitAdapter;
use skald_vcs::{DiffOptions, StageMode, VcsAdapter};

pub struct CommitOptions {
    pub show_prompt: bool,
    pub auto: bool,
    pub message_only: bool,
    pub count: usize,
    pub stage_tracked: bool,
    pub stage_all: bool,
    pub amend: bool,
    pub context: Option<String>,
    pub context_file: Option<PathBuf>,
    pub dry_run: bool,
    pub format: OutputFormat,
    pub is_tty: bool,
}

pub fn run_commit(opts: CommitOptions, config: &ResolvedConfig) -> i32 {
    // 1. Detect git repo
    let git = match GitAdapter::detect() {
        Ok(g) => g,
        Err(e) => {
            cliclack::log::error(format!("Not in a git repository: {e}")).ok();
            return 1;
        }
    };

    let branch = git.get_current_branch().unwrap_or_else(|_| "HEAD".to_string());

    // 2. show_prompt: render template with real branch but placeholder diff, print, exit
    if opts.show_prompt {
        return run_show_prompt(&branch, &config.language);
    }

    // If neither --auto nor --message-only, interactive mode is not yet ready
    if !opts.auto && !opts.message_only {
        println!("Interactive mode not yet implemented — use --auto or --message-only");
        return 0;
    }

    // 3. Stage if requested
    if opts.stage_all {
        if let Err(e) = git.stage(StageMode::All) {
            cliclack::log::error(format!("Failed to stage files: {e}")).ok();
            return 1;
        }
    } else if opts.stage_tracked
        && let Err(e) = git.stage(StageMode::Tracked)
    {
        cliclack::log::error(format!("Failed to stage files: {e}")).ok();
        return 1;
    }

    // 4. Check for staged changes
    match git.has_staged_changes() {
        Ok(false) => {
            cliclack::log::error(
                "No staged changes found. Stage files first with `git add` or use `-a`/`-A`.",
            )
            .ok();
            return 1;
        }
        Err(e) => {
            cliclack::log::error(format!("Failed to check staged changes: {e}")).ok();
            return 1;
        }
        Ok(true) => {}
    }

    // 5. Get diff (staged)
    let diff_result = match git.get_diff(&DiffOptions { staged: true, exclude_patterns: vec![] }) {
        Ok(d) => d,
        Err(e) => {
            cliclack::log::error(format!("Failed to get diff: {e}")).ok();
            return 1;
        }
    };

    // 6. Load context
    let context = load_context(&opts.context, &opts.context_file);

    // 7. Determine effective mode — message_only if explicit or if -n given without --auto
    let effective_message_only = opts.message_only || (opts.count != 3 && !opts.auto);

    // 8. Build PromptContext
    let files_changed = extract_files_from_stat(&diff_result.stat);
    let prompt_ctx = PromptContext::new()
        .set("branch", &branch)
        .set("diff_stat", &diff_result.stat)
        .set("context", context.as_deref().unwrap_or(""))
        .set("language", &config.language)
        .set("num_suggestions", &opts.count.to_string())
        .set("files_changed", &files_changed);

    // 9. Resolve + render commit-title template
    let template = match resolve_template("commit-title", None, None) {
        Ok(t) => t,
        Err(e) => {
            cliclack::log::error(format!("Failed to resolve prompt template: {e}")).ok();
            return 1;
        }
    };

    let rendered_prompt = match render_prompt(&template, &prompt_ctx) {
        Ok(r) => r,
        Err(e) => {
            cliclack::log::error(format!("Failed to render prompt: {e}")).ok();
            return 1;
        }
    };

    // 10. Prepend system message
    let system_template = resolve_template("system", None, None).unwrap_or_default();
    let system_msg = render_prompt(&system_template, &prompt_ctx).unwrap_or(system_template);
    let full_prompt = format!("{system_msg}\n\n{rendered_prompt}");

    // 11. Build CommitContext
    let commit_ctx = CommitContext {
        diff: diff_result.diff.clone(),
        stat: diff_result.stat.clone(),
        rendered_prompt: full_prompt,
        extra_context: context.clone(),
    };

    // 12. Create provider
    let model = config.providers.get(&config.provider).and_then(|p| p.model.clone());
    let provider = ClaudeCliProvider::new(model);

    // 13. Show spinner, call provider
    let count = if opts.auto { 1 } else { opts.count };

    let sp = if opts.is_tty {
        let s = cliclack::spinner();
        s.start("Generating commit messages...");
        Some(s)
    } else {
        None
    };

    let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => {
            if let Some(s) = sp {
                s.stop("Failed");
            }
            cliclack::log::error(format!("Failed to create async runtime: {e}")).ok();
            return 1;
        }
    };

    let messages = match rt.block_on(provider.generate_commit_messages(&commit_ctx, count)) {
        Ok(msgs) => {
            if let Some(s) = sp {
                s.stop("Done");
            }
            msgs
        }
        Err(e) => {
            if let Some(s) = sp {
                s.stop("Failed");
            }
            cliclack::log::error(format!("AI generation failed: {e}")).ok();
            return 1;
        }
    };

    if messages.is_empty() {
        cliclack::log::error("No commit messages generated.").ok();
        return 1;
    }

    // 14. message_only mode: print messages and exit
    if effective_message_only {
        return render_messages(&messages, opts.format, opts.is_tty);
    }

    // 15. auto mode: take first message, commit (or amend)
    if opts.auto {
        let msg = &messages[0];

        if opts.dry_run {
            println!("Would commit with message: {msg}");
            return 0;
        }

        let result = if opts.amend { git.commit_amend(msg) } else { git.commit(msg) };

        return match result {
            Ok(output) => {
                cliclack::log::success(format!(
                    "{}committed: {msg}",
                    if opts.amend { "amended and " } else { "" }
                ))
                .ok();
                tracing::debug!(%output, "git commit output");
                0
            }
            Err(e) => {
                cliclack::log::error(format!("Commit failed: {e}")).ok();
                1
            }
        };
    }

    // 16. dry_run without auto (shouldn't normally reach here, but handle it)
    if opts.dry_run {
        for (i, msg) in messages.iter().enumerate() {
            println!("  {}. {msg}", i + 1);
        }
        println!("\n(dry run — nothing committed)");
        return 0;
    }

    0
}

fn run_show_prompt(branch: &str, language: &str) -> i32 {
    let ctx = PromptContext::new()
        .set("branch", branch)
        .set("diff_stat", "<diff will appear here at generation time>")
        .set("context", "")
        .set("language", language)
        .set("num_suggestions", "3")
        .set("files_changed", "<files will appear here>");

    match resolve_template("commit-title", None, None) {
        Ok(template) => match render_prompt(&template, &ctx) {
            Ok(rendered) => {
                print!("{rendered}");
                0
            }
            Err(e) => {
                cliclack::log::error(e.to_string()).ok();
                1
            }
        },
        Err(e) => {
            cliclack::log::error(e.to_string()).ok();
            1
        }
    }
}

fn load_context(context: &Option<String>, context_file: &Option<PathBuf>) -> Option<String> {
    if let Some(ctx) = context {
        return Some(ctx.clone());
    }
    if let Some(path) = context_file {
        match std::fs::read_to_string(path) {
            Ok(contents) => return Some(contents),
            Err(e) => {
                cliclack::log::error(format!(
                    "Failed to read context file '{}': {e}",
                    path.display()
                ))
                .ok();
            }
        }
    }
    None
}

fn extract_files_from_stat(stat: &str) -> String {
    stat.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            // stat lines look like: " src/main.rs | 10 ++++------"
            // the summary line contains "changed" — skip it
            if trimmed.is_empty() || trimmed.contains("changed") {
                return None;
            }
            trimmed.split('|').next().map(|p| p.trim().to_string())
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_messages(messages: &[String], format: OutputFormat, is_tty: bool) -> i32 {
    match format {
        OutputFormat::Json => {
            let json = if is_tty {
                serde_json::to_string_pretty(messages)
            } else {
                serde_json::to_string(messages)
            }
            .unwrap_or_else(|_| "[]".to_string());
            println!("{json}");
        }
        _ => {
            for msg in messages {
                println!("{msg}");
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_files_single() {
        let stat = " src/main.rs | 10 ++++------\n 1 file changed, 4 insertions(+), 6 deletions(-)";
        assert_eq!(extract_files_from_stat(stat), "src/main.rs");
    }

    #[test]
    fn extract_files_multiple() {
        let stat = " src/main.rs | 10 ++++------\n src/lib.rs  |  5 +++--\n 2 files changed, 7 insertions(+), 8 deletions(-)";
        assert_eq!(extract_files_from_stat(stat), "src/main.rs, src/lib.rs");
    }

    #[test]
    fn extract_files_empty() {
        assert_eq!(extract_files_from_stat(""), "");
    }

    #[test]
    fn load_context_from_string() {
        let ctx = load_context(&Some("hello".to_string()), &None);
        assert_eq!(ctx, Some("hello".to_string()));
    }

    #[test]
    fn load_context_none() {
        let ctx = load_context(&None, &None);
        assert_eq!(ctx, None);
    }

    #[test]
    fn load_context_from_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "file context").unwrap();
        let ctx = load_context(&None, &Some(tmp.path().to_path_buf()));
        assert_eq!(ctx, Some("file context".to_string()));
    }

    #[test]
    fn load_context_string_takes_priority() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "file context").unwrap();
        let ctx =
            load_context(&Some("string context".to_string()), &Some(tmp.path().to_path_buf()));
        assert_eq!(ctx, Some("string context".to_string()));
    }
}
