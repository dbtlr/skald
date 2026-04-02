use std::path::PathBuf;

use crate::engine::config::schema::ResolvedConfig;
use crate::engine::output::OutputFormat;
use crate::engine::prompts::{PromptContext, render_prompt, resolve_template};
use crate::providers::{CliProvider, CommitContext, Provider, get_provider_config};
use crate::vcs::git::GitAdapter;
use crate::vcs::{DiffOptions, StageMode, VcsAdapter};

pub struct CommitOptions {
    pub yes: bool,
    pub count: usize,
    pub all: bool,
    pub include_untracked: bool,
    pub amend: bool,
    pub context: Option<String>,
    pub context_file: Option<PathBuf>,
    pub dry_run: bool,
    pub body: bool,
    pub format: OutputFormat,
    pub is_tty: bool,
    pub provider_name: String,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
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

    // 2. Determine staging mode — deferred staging uses a temp index so
    //    cancellation leaves the real index untouched.
    let deferred_stage = if opts.include_untracked {
        Some(StageMode::All)
    } else if opts.all {
        Some(StageMode::Tracked)
    } else {
        None
    };

    let diff_options = DiffOptions { staged: true, exclude_patterns: vec![] };

    // 3. Get diff — either from real index or previewed via temp index
    let diff_result = if let Some(mode) = deferred_stage {
        match git.preview_staged_diff(mode, &diff_options) {
            Ok(d) if d.diff.is_empty() => {
                // Preview found nothing to stage
                if let Some(code) = handle_no_staged_changes(&git, opts.is_tty) {
                    return code;
                }
                // User chose to stage interactively — now get the real diff
                match git.get_diff(&diff_options) {
                    Ok(d) => d,
                    Err(e) => {
                        cliclack::log::error(format!("Failed to get diff: {e}")).ok();
                        return 1;
                    }
                }
            }
            Ok(d) => d,
            Err(e) => {
                cliclack::log::error(format!("Failed to preview staging: {e}")).ok();
                return 1;
            }
        }
    } else {
        // No -a / --include-untracked — check real staged changes
        match git.has_staged_changes() {
            Ok(false) => {
                if let Some(code) = handle_no_staged_changes(&git, opts.is_tty) {
                    return code;
                }
                // Verify staging actually produced changes
                match git.has_staged_changes() {
                    Ok(true) => {}
                    _ => {
                        cliclack::log::error("Still no staged changes after staging.").ok();
                        return 1;
                    }
                }
            }
            Err(e) => {
                cliclack::log::error(format!("Failed to check staged changes: {e}")).ok();
                return 1;
            }
            Ok(true) => {}
        }
        match git.get_diff(&diff_options) {
            Ok(d) => d,
            Err(e) => {
                cliclack::log::error(format!("Failed to get diff: {e}")).ok();
                return 1;
            }
        }
    };

    // 6. Load context
    let context = load_context(&opts.context, &opts.context_file);

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

    tracing::trace!(prompt = %full_prompt, "rendered commit prompt");

    // 11. Build CommitContext
    let commit_ctx = CommitContext {
        diff: diff_result.diff.clone(),
        stat: diff_result.stat.clone(),
        rendered_prompt: full_prompt,
        extra_context: context.clone(),
    };

    // 12. Create provider
    let provider_config = match get_provider_config(&opts.provider_name) {
        Some(c) => c,
        None => {
            cliclack::log::error(format!(
                "Unknown provider '{}'. Available: {}",
                opts.provider_name,
                crate::providers::available_provider_names().join(", ")
            ))
            .ok();
            return 1;
        }
    };
    let provider = CliProvider::new(provider_config, opts.model.clone());

    // 13. Show spinner, call provider
    let count = if opts.yes { 1 } else { opts.count };

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

    // 14. dry_run mode: print messages and exit
    if opts.dry_run {
        return render_messages(&messages, opts.format, opts.is_tty);
    }

    // 15. yes mode: take first message, commit (or amend)
    if opts.yes {
        let msg = &messages[0];
        if opts.body {
            if let Some(body) = generate_body(
                msg,
                &diff_result.stat,
                context.as_deref(),
                &config.language,
                &diff_result.diff,
                &provider,
                &rt,
                opts.is_tty,
            ) {
                return do_commit_with_body(
                    &git,
                    msg,
                    &body,
                    opts.amend,
                    opts.dry_run,
                    deferred_stage,
                );
            }
            cliclack::log::warning("Body generation failed, committing with title only.").ok();
        }
        return do_commit(&git, msg, opts.amend, false, deferred_stage);
    }

    // 16. Interactive mode
    let mut state = InteractiveState {
        messages,
        git: &git,
        provider: &provider,
        rt: &rt,
        diff: &diff_result.diff,
        diff_stat: &diff_result.stat,
        branch: &branch,
        context,
        language: &config.language,
        count,
        amend: opts.amend,
        dry_run: opts.dry_run,
        is_tty: opts.is_tty,
        deferred_stage,
    };
    run_interactive(&mut state)
}

/// Handle the case where no changes are staged.
///
/// Returns `Some(exit_code)` if the commit flow should stop,
/// or `None` if staging succeeded and the flow should continue.
fn handle_no_staged_changes(git: &GitAdapter, is_tty: bool) -> Option<i32> {
    // Check if there are unstaged changes we could offer to stage.
    let has_unstaged = match git.has_unstaged_changes() {
        Ok(v) => v,
        Err(e) => {
            cliclack::log::error(format!("Failed to check unstaged changes: {e}")).ok();
            return Some(1);
        }
    };

    if !has_unstaged {
        cliclack::log::error("No staged or unstaged changes found.").ok();
        return Some(1);
    }

    // Non-interactive: can't prompt, just tell the user what to do.
    if !is_tty {
        cliclack::log::error(
            "No staged changes found. Use `-a`/`--all` to stage tracked files or `--include-untracked` to include new files.",
        )
        .ok();
        return Some(1);
    }

    // Interactive: offer to stage.
    let selection = cliclack::select("No staged changes. How would you like to proceed?")
        .item("all", "Stage all (--include-untracked)", "includes untracked files")
        .item("tracked", "Stage tracked (-a/--all)", "only already-tracked files")
        .item("abort", "Abort", "")
        .interact();

    match selection {
        Ok("all") => {
            if let Err(e) = git.stage(StageMode::All) {
                cliclack::log::error(format!("Failed to stage files: {e}")).ok();
                return Some(1);
            }
            None
        }
        Ok("tracked") => {
            if let Err(e) = git.stage(StageMode::Tracked) {
                cliclack::log::error(format!("Failed to stage files: {e}")).ok();
                return Some(1);
            }
            None
        }
        Ok(_) | Err(_) => {
            // "abort" or cancelled
            Some(130)
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

struct InteractiveState<'a> {
    messages: Vec<String>,
    git: &'a GitAdapter,
    provider: &'a CliProvider,
    rt: &'a tokio::runtime::Runtime,
    diff: &'a str,
    diff_stat: &'a str,
    branch: &'a str,
    context: Option<String>,
    language: &'a str,
    count: usize,
    amend: bool,
    dry_run: bool,
    is_tty: bool,
    deferred_stage: Option<StageMode>,
}

fn run_interactive(state: &mut InteractiveState) -> i32 {
    use crate::ui::carousel::{CarouselResult, show_carousel};

    loop {
        let result = match show_carousel(&state.messages) {
            Ok(r) => r,
            Err(e) => {
                cliclack::log::error(format!("Carousel error: {e}")).ok();
                return 1;
            }
        };

        match result {
            CarouselResult::Accept(idx) => {
                return do_commit(
                    state.git,
                    &state.messages[idx],
                    state.amend,
                    state.dry_run,
                    state.deferred_stage,
                );
            }
            CarouselResult::Edit(idx) => {
                let edited: Result<String, _> = cliclack::input("Edit commit message:")
                    .default_input(&state.messages[idx])
                    .interact();
                match edited {
                    Ok(msg) if !msg.is_empty() => {
                        return do_commit(
                            state.git,
                            &msg,
                            state.amend,
                            state.dry_run,
                            state.deferred_stage,
                        );
                    }
                    Ok(_) => {
                        cliclack::log::warning("Empty message — returning to carousel.").ok();
                        continue;
                    }
                    Err(_) => {
                        cliclack::log::info("Aborted.").ok();
                        return 130;
                    }
                }
            }
            CarouselResult::Extend(idx) => {
                let code = handle_extend(state, idx);
                if let Some(c) = code {
                    return c;
                }
                // None means "back to carousel"
                continue;
            }
            CarouselResult::Menu(idx) => {
                let choice = cliclack::select("What would you like to do?")
                    .item("accept", "Accept", format!("commit: {}", &state.messages[idx]))
                    .item("extend", "Extend", "generate extended description")
                    .item("context", "Context", "add context and regenerate messages")
                    .item("amend", "Amend", "commit with --amend")
                    .item("abort", "Abort", "exit without committing")
                    .interact();

                match choice {
                    Ok("accept") => {
                        return do_commit(
                            state.git,
                            &state.messages[idx],
                            false,
                            state.dry_run,
                            state.deferred_stage,
                        );
                    }
                    Ok("extend") => {
                        let code = handle_extend(state, idx);
                        if let Some(c) = code {
                            return c;
                        }
                        continue;
                    }
                    Ok("context") => {
                        handle_context_regeneration(state);
                        continue;
                    }
                    Ok("amend") => {
                        return do_commit(
                            state.git,
                            &state.messages[idx],
                            true,
                            state.dry_run,
                            state.deferred_stage,
                        );
                    }
                    Ok("abort") | Err(_) => {
                        cliclack::log::info("Aborted.").ok();
                        return 130;
                    }
                    _ => continue,
                }
            }
            CarouselResult::Abort => {
                cliclack::log::info("Aborted.").ok();
                return 130;
            }
        }
    }
}

/// Handle the extend flow: generate body, offer accept/edit/back.
/// Returns `Some(exit_code)` to exit, or `None` to return to carousel.
fn handle_extend(state: &InteractiveState, idx: usize) -> Option<i32> {
    let title = &state.messages[idx];
    let body = generate_body(
        title,
        state.diff_stat,
        state.context.as_deref(),
        state.language,
        state.diff,
        state.provider,
        state.rt,
        state.is_tty,
    );

    let body = match body {
        Some(b) => b,
        None => {
            cliclack::log::warning("Failed to generate extended description.").ok();
            return None;
        }
    };

    cliclack::log::info(format!("Extended description:\n{body}")).ok();

    let choice = cliclack::select("What would you like to do with this description?")
        .item("accept", "Accept", "commit with title + body")
        .item("edit", "Edit", "edit the body before committing")
        .item("back", "Back", "return to carousel")
        .interact();

    match choice {
        Ok("accept") => Some(do_commit_with_body(
            state.git,
            title,
            &body,
            state.amend,
            state.dry_run,
            state.deferred_stage,
        )),
        Ok("edit") => {
            let edited: Result<String, _> =
                cliclack::input("Edit description:").default_input(&body).interact();
            match edited {
                Ok(edited_body) if !edited_body.is_empty() => Some(do_commit_with_body(
                    state.git,
                    title,
                    &edited_body,
                    state.amend,
                    state.dry_run,
                    state.deferred_stage,
                )),
                Ok(_) => {
                    cliclack::log::warning("Empty body — returning to carousel.").ok();
                    None
                }
                Err(_) => {
                    cliclack::log::info("Aborted.").ok();
                    Some(130)
                }
            }
        }
        Ok("back") | Err(_) => None,
        _ => None,
    }
}

/// Handle context regeneration: prompt for context, regenerate messages.
fn handle_context_regeneration(state: &mut InteractiveState) {
    let input: Result<String, _> = cliclack::input("Add context for regeneration:")
        .placeholder("e.g. this is a bugfix for the login flow")
        .interact();

    let new_context = match input {
        Ok(ctx) if !ctx.is_empty() => ctx,
        Ok(_) => {
            cliclack::log::warning("No context provided — keeping current messages.").ok();
            return;
        }
        Err(_) => {
            cliclack::log::info("Cancelled.").ok();
            return;
        }
    };

    // Update context — append to existing or set new
    state.context = Some(match &state.context {
        Some(existing) => format!("{existing}\n{new_context}"),
        None => new_context,
    });

    if let Some(new_messages) = regenerate_messages(state) {
        state.messages = new_messages;
        cliclack::log::success("Messages regenerated with new context.").ok();
    } else {
        cliclack::log::warning("Regeneration failed — keeping current messages.").ok();
    }
}

/// Regenerate commit messages using the current state (including updated context).
fn regenerate_messages(state: &InteractiveState) -> Option<Vec<String>> {
    let files_changed = extract_files_from_stat(state.diff_stat);
    let prompt_ctx = PromptContext::new()
        .set("branch", state.branch)
        .set("diff_stat", state.diff_stat)
        .set("context", state.context.as_deref().unwrap_or(""))
        .set("language", state.language)
        .set("num_suggestions", &state.count.to_string())
        .set("files_changed", &files_changed);

    let template = resolve_template("commit-title", None, None).ok()?;
    let rendered_prompt = render_prompt(&template, &prompt_ctx).ok()?;

    let system_template = resolve_template("system", None, None).unwrap_or_default();
    let system_msg = render_prompt(&system_template, &prompt_ctx).unwrap_or(system_template);
    let full_prompt = format!("{system_msg}\n\n{rendered_prompt}");

    let commit_ctx = CommitContext {
        diff: state.diff.to_string(),
        stat: state.diff_stat.to_string(),
        rendered_prompt: full_prompt,
        extra_context: state.context.clone(),
    };

    let sp = if state.is_tty {
        let s = cliclack::spinner();
        s.start("Regenerating commit messages...");
        Some(s)
    } else {
        None
    };

    match state.rt.block_on(state.provider.generate_commit_messages(&commit_ctx, state.count)) {
        Ok(msgs) if !msgs.is_empty() => {
            if let Some(s) = sp {
                s.stop("Done");
            }
            Some(msgs)
        }
        Ok(_) => {
            if let Some(s) = sp {
                s.stop("Failed");
            }
            cliclack::log::warning("No messages returned.").ok();
            None
        }
        Err(e) => {
            if let Some(s) = sp {
                s.stop("Failed");
            }
            cliclack::log::warning(format!("Regeneration failed: {e}")).ok();
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn generate_body(
    title: &str,
    diff_stat: &str,
    context: Option<&str>,
    language: &str,
    diff: &str,
    provider: &CliProvider,
    rt: &tokio::runtime::Runtime,
    is_tty: bool,
) -> Option<String> {
    // 1. Build PromptContext with title, diff_stat, context, language
    let prompt_ctx = PromptContext::new()
        .set("title", title)
        .set("diff_stat", diff_stat)
        .set("context", context.unwrap_or(""))
        .set("language", language);

    // 2. Resolve + render "commit-body" template
    let template = match resolve_template("commit-body", None, None) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(%e, "failed to resolve commit-body template");
            return None;
        }
    };

    let rendered_prompt = match render_prompt(&template, &prompt_ctx) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(%e, "failed to render commit-body template");
            return None;
        }
    };

    // 3. Prepend system message
    let system_template = resolve_template("system", None, None).unwrap_or_default();
    let system_msg = render_prompt(&system_template, &prompt_ctx).unwrap_or(system_template);
    let full_prompt = format!("{system_msg}\n\n{rendered_prompt}");

    // 4. Build CommitContext with diff, stat, rendered prompt
    let ctx = CommitContext {
        diff: diff.to_string(),
        stat: diff_stat.to_string(),
        rendered_prompt: full_prompt,
        extra_context: context.map(|s| s.to_string()),
    };

    // 5. Show spinner, call provider
    let sp = if is_tty {
        let s = cliclack::spinner();
        s.start("Generating extended description...");
        Some(s)
    } else {
        None
    };

    match rt.block_on(provider.generate_commit_messages(&ctx, 1)) {
        Ok(msgs) => {
            if let Some(s) = sp {
                s.stop("Done");
            }
            // 6. Return the body text (join all lines — the body may be multi-line)
            msgs.into_iter().next().filter(|s| !s.is_empty())
        }
        Err(e) => {
            if let Some(s) = sp {
                s.stop("Failed");
            }
            tracing::warn!(%e, "body generation failed");
            None
        }
    }
}

/// Stage files for real if deferred staging was used.
/// Returns `Some(exit_code)` on failure, `None` on success.
fn apply_deferred_stage(git: &GitAdapter, deferred_stage: Option<StageMode>) -> Option<i32> {
    if let Some(mode) = deferred_stage
        && let Err(e) = git.stage(mode)
    {
        cliclack::log::error(format!("Failed to stage files: {e}")).ok();
        return Some(1);
    }
    None
}

fn do_commit_with_body(
    git: &GitAdapter,
    title: &str,
    body: &str,
    amend: bool,
    dry_run: bool,
    deferred_stage: Option<StageMode>,
) -> i32 {
    if dry_run {
        cliclack::log::info(format!("Would commit:\n  Title: {title}\n  Body:\n{body}")).ok();
        return 0;
    }

    if let Some(code) = apply_deferred_stage(git, deferred_stage) {
        return code;
    }

    let result = if amend {
        git.commit_amend_with_body(title, body)
    } else {
        git.commit_with_body(title, body)
    };

    match result {
        Ok(output) => {
            cliclack::log::success(format!("Committed: {title}")).ok();
            cliclack::log::info("  (with extended description)").ok();
            tracing::debug!(%output, "git commit output");
            0
        }
        Err(e) => {
            cliclack::log::error(format!("Commit failed: {e}")).ok();
            1
        }
    }
}

fn do_commit(
    git: &GitAdapter,
    message: &str,
    amend: bool,
    dry_run: bool,
    deferred_stage: Option<StageMode>,
) -> i32 {
    if dry_run {
        cliclack::log::info(format!("Would commit: {message}")).ok();
        return 0;
    }

    if let Some(code) = apply_deferred_stage(git, deferred_stage) {
        return code;
    }

    let result = if amend { git.commit_amend(message) } else { git.commit(message) };

    match result {
        Ok(output) => {
            cliclack::log::success(format!(
                "{}committed: {message}",
                if amend { "amended and " } else { "" }
            ))
            .ok();
            tracing::debug!(%output, "git commit output");
            0
        }
        Err(e) => {
            cliclack::log::error(format!("Commit failed: {e}")).ok();
            1
        }
    }
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
