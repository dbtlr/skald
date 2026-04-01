use crate::engine::config::schema::ResolvedConfig;
use crate::engine::output::OutputFormat;
use crate::engine::prompts::{PromptContext, render_prompt, resolve_template};
use crate::platform::{CreatePrRequest, PlatformAdapter, detect_platform};
use crate::providers::{CliProvider, PrContent, PrContext, Provider, get_provider_config};
use crate::vcs::git::GitAdapter;
use crate::vcs::{DiffOptions, DiffResult, VcsAdapter};

pub struct PrOptions {
    pub yes: bool,
    pub dry_run: bool,
    pub draft: bool,
    pub push: bool,
    pub base: Option<String>,
    pub count: usize,
    pub context: Option<String>,
    pub context_file: Option<std::path::PathBuf>,
    pub format: OutputFormat,
    pub is_tty: bool,
    pub provider_name: String,
    pub model: Option<String>,
}

/// Determine the source ref for diff/log commands based on push flag and upstream state.
fn resolve_source_ref(git: &GitAdapter, push: bool) -> String {
    if push {
        return "HEAD".to_string();
    }
    match git.get_upstream_ref() {
        Ok(_) => "@{u}".to_string(),
        Err(_) => "HEAD".to_string(),
    }
}

/// Extract common prompt building + AI generation into a reusable function.
#[allow(clippy::too_many_arguments)]
fn generate_pr_contents(
    branch: &str,
    target: &str,
    diff_result: &DiffResult,
    commit_log: &str,
    context: Option<&str>,
    count: usize,
    config: &ResolvedConfig,
    is_tty: bool,
    provider_name: &str,
    model: Option<String>,
) -> Result<Vec<PrContent>, i32> {
    let prompt_ctx = PromptContext::new()
        .set("branch", branch)
        .set("target_branch", target)
        .set("diff_stat", &diff_result.stat)
        .set("context", context.unwrap_or(""))
        .set("language", &config.language)
        .set("num_suggestions", &count.to_string())
        .set("commit_log", commit_log);

    let template = match resolve_template("pr", None, None) {
        Ok(t) => t,
        Err(e) => {
            cliclack::log::error(format!("Failed to resolve prompt template: {e}")).ok();
            return Err(1);
        }
    };

    let rendered_prompt = match render_prompt(&template, &prompt_ctx) {
        Ok(r) => r,
        Err(e) => {
            cliclack::log::error(format!("Failed to render prompt: {e}")).ok();
            return Err(1);
        }
    };

    let system_template = resolve_template("system", None, None).unwrap_or_default();
    let system_msg = render_prompt(&system_template, &prompt_ctx).unwrap_or(system_template);
    let full_prompt = format!("{system_msg}\n\n{rendered_prompt}");

    tracing::trace!(prompt = %full_prompt, "rendered PR prompt");

    let pr_ctx = PrContext {
        diff: diff_result.diff.clone(),
        commit_log: commit_log.to_string(),
        target_branch: target.to_string(),
        rendered_prompt: full_prompt,
        extra_context: context.map(|s| s.to_string()),
    };

    let provider_config = match get_provider_config(provider_name) {
        Some(c) => c,
        None => {
            cliclack::log::error(format!(
                "Unknown provider '{}'. Available: {}",
                provider_name,
                crate::providers::available_provider_names().join(", ")
            ))
            .ok();
            return Err(1);
        }
    };
    let provider = CliProvider::new(provider_config, model);

    let sp = if is_tty {
        let s = cliclack::spinner();
        s.start("Generating content...");
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
            return Err(1);
        }
    };

    let contents = match rt.block_on(provider.generate_pr_content(&pr_ctx, count)) {
        Ok(c) => {
            if let Some(s) = sp {
                s.stop("Done");
            }
            c
        }
        Err(e) => {
            if let Some(s) = sp {
                s.stop("Failed");
            }
            cliclack::log::error(format!("AI generation failed: {e}")).ok();
            return Err(1);
        }
    };

    if contents.is_empty() {
        cliclack::log::error("No content generated.").ok();
        return Err(1);
    }

    Ok(contents)
}

pub fn run_pr(opts: PrOptions, config: &ResolvedConfig) -> i32 {
    // 1. Detect git repo
    let git = match GitAdapter::detect() {
        Ok(g) => g,
        Err(e) => {
            cliclack::log::error(format!("Not in a git repository: {e}")).ok();
            return 1;
        }
    };

    // 2. Get current branch
    let branch = git.get_current_branch().unwrap_or_else(|_| "HEAD".to_string());

    // 3. Detect platform early — needed to check for existing PR
    let remote_url = match git.get_remote_url() {
        Ok(url) => url,
        Err(e) => {
            cliclack::log::error(format!("Failed to get remote URL: {e}")).ok();
            return 1;
        }
    };

    let platform = match detect_platform(&remote_url, Some(config.platform.as_str())) {
        Some(p) => p,
        None => {
            cliclack::log::error(
                "Could not detect platform from remote URL. Set `platform: github` or `platform: gitlab` in your config.",
            )
            .ok();
            return 1;
        }
    };

    // 4. Check for existing PR — determines create vs update behavior
    let label = platform.pr_label();
    let existing_pr = match platform.pr_exists(&branch) {
        Ok(pr) => pr,
        Err(e) => {
            cliclack::log::error(format!("Failed to check for existing {label}: {e}")).ok();
            return 1;
        }
    };
    let is_update = existing_pr.is_some();

    // 5. Resolve target branch: --base flag -> existing PR base -> config -> "main"
    let target = opts.base.clone().unwrap_or_else(|| {
        existing_pr
            .as_ref()
            .map(|pr| pr.base_branch.clone())
            .unwrap_or_else(|| config.pr_target.clone())
    });

    // 6. Get branch diff and commit log
    let source = resolve_source_ref(&git, opts.push);

    let diff_result = match git.get_branch_diff(
        &target,
        &source,
        &DiffOptions { staged: false, exclude_patterns: vec![] },
    ) {
        Ok(d) => d,
        Err(e) => {
            cliclack::log::error(format!("Failed to get branch diff: {e}")).ok();
            return 1;
        }
    };

    let commit_log = match git.get_commit_log(&target, &source) {
        Ok(log) => log,
        Err(e) => {
            cliclack::log::error(format!("Failed to get commit log: {e}")).ok();
            return 1;
        }
    };

    if commit_log.trim().is_empty() {
        cliclack::log::error(format!("No commits found ahead of '{target}'.")).ok();
        return 1;
    }

    // 7. Load context and generate PR contents
    let context = load_context(&opts.context, &opts.context_file);
    let count = if opts.yes { 1 } else { opts.count };
    let contents = match generate_pr_contents(
        &branch,
        &target,
        &diff_result,
        &commit_log,
        context.as_deref(),
        count,
        config,
        opts.is_tty,
        &opts.provider_name,
        opts.model.clone(),
    ) {
        Ok(c) => c,
        Err(code) => return code,
    };

    // 8. dry_run: render payload
    if opts.dry_run {
        return render_dry_run(&contents, opts.format, opts.is_tty);
    }

    // 9. yes mode: auto-create or auto-update
    if opts.yes {
        if is_update {
            return do_update_pr(
                &git,
                platform.as_ref(),
                &branch,
                &contents[0],
                opts.push,
                opts.is_tty,
            );
        }
        return create_pr(
            platform.as_ref(),
            &git,
            &contents[0],
            &target,
            opts.draft,
            opts.push,
            opts.is_tty,
        );
    }

    // 10. Interactive mode — pass is_update so the UI offers the right actions
    run_interactive_pr(
        &git,
        platform.as_ref(),
        &branch,
        &target,
        &diff_result,
        &commit_log,
        contents,
        &opts,
        config,
        is_update,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_interactive_pr(
    git: &GitAdapter,
    platform: &dyn PlatformAdapter,
    branch: &str,
    target: &str,
    diff_result: &DiffResult,
    commit_log: &str,
    mut contents: Vec<PrContent>,
    opts: &PrOptions,
    config: &ResolvedConfig,
    is_update: bool,
) -> i32 {
    use crate::ui::carousel::{CarouselResult, show_carousel};

    let mut context = opts.context.clone();

    loop {
        // Stage 1: Title Carousel
        let titles: Vec<String> = contents.iter().map(|c| c.title.clone()).collect();

        let carousel_result = match show_carousel(&titles) {
            Ok(r) => r,
            Err(e) => {
                cliclack::log::error(format!("Carousel error: {e}")).ok();
                return 1;
            }
        };

        let selected_idx = match carousel_result {
            CarouselResult::Accept(idx) => idx,
            CarouselResult::Edit(idx) => {
                let edited: Result<String, _> = cliclack::input("Edit PR title:")
                    .default_input(&contents[idx].title)
                    .interact();
                match edited {
                    Ok(title) if !title.is_empty() => {
                        contents[idx].title = title;
                        idx
                    }
                    Ok(_) => {
                        cliclack::log::warning("Empty title — returning to carousel.").ok();
                        continue;
                    }
                    Err(_) => {
                        cliclack::log::info("Aborted.").ok();
                        return 130;
                    }
                }
            }
            CarouselResult::Extend(idx) | CarouselResult::Menu(idx) => {
                // Context regeneration from carousel
                if let Some(new_contents) = handle_context_regeneration(
                    branch,
                    target,
                    diff_result,
                    commit_log,
                    &mut context,
                    opts.count,
                    config,
                    opts.is_tty,
                    &opts.provider_name,
                    opts.model.clone(),
                ) {
                    contents = new_contents;
                }
                // Ignore idx — return to carousel with updated or same contents
                let _ = idx;
                continue;
            }
            CarouselResult::Abort => {
                cliclack::log::info("Aborted.").ok();
                return 130;
            }
        };

        // Stage 2: Confirmation Menu
        let exit_code = run_confirmation_menu(
            git,
            platform,
            branch,
            target,
            diff_result,
            commit_log,
            &mut contents,
            selected_idx,
            &mut context,
            opts,
            config,
            is_update,
        );

        match exit_code {
            ConfirmationResult::Exit(code) => return code,
            ConfirmationResult::BackToCarousel => continue,
        }
    }
}

enum ConfirmationResult {
    Exit(i32),
    BackToCarousel,
}

#[allow(clippy::too_many_arguments)]
fn run_confirmation_menu(
    git: &GitAdapter,
    platform: &dyn PlatformAdapter,
    branch: &str,
    target: &str,
    diff_result: &DiffResult,
    commit_log: &str,
    contents: &mut Vec<PrContent>,
    idx: usize,
    context: &mut Option<String>,
    opts: &PrOptions,
    config: &ResolvedConfig,
    is_update: bool,
) -> ConfirmationResult {
    use crate::ui::editor::edit_in_editor;

    let mut title = contents[idx].title.clone();
    let mut body = contents[idx].body.clone();

    loop {
        // Show preview
        cliclack::log::info(format!("Title: {title}\n\n{body}")).ok();

        // Build menu based on mode
        let action = if is_update {
            cliclack::select("What would you like to do?")
                .item("update", "Update", "push and update the PR")
                .item("edit_title", "Edit title", "edit the PR title inline")
                .item("edit_body", "Edit body", "edit the PR body in your editor")
                .item("context", "Context", "add context and regenerate")
                .item("abort", "Abort", "exit without updating")
                .interact()
        } else {
            cliclack::select("What would you like to do?")
                .item("create", "Create", "create the PR")
                .item("draft", "Draft", "create as draft PR")
                .item("edit_title", "Edit title", "edit the PR title inline")
                .item("edit_body", "Edit body", "edit the PR body in your editor")
                .item("context", "Context", "add context and regenerate")
                .item("abort", "Abort", "exit without creating")
                .interact()
        };

        match action {
            Ok("create") => {
                let content = PrContent { title: title.clone(), body: body.clone() };
                return ConfirmationResult::Exit(create_pr(
                    platform,
                    git,
                    &content,
                    target,
                    false,
                    opts.push,
                    opts.is_tty,
                ));
            }
            Ok("draft") => {
                let content = PrContent { title: title.clone(), body: body.clone() };
                return ConfirmationResult::Exit(create_pr(
                    platform,
                    git,
                    &content,
                    target,
                    true,
                    opts.push,
                    opts.is_tty,
                ));
            }
            Ok("update") => {
                let content = PrContent { title: title.clone(), body: body.clone() };
                return ConfirmationResult::Exit(do_update_pr(
                    git,
                    platform,
                    branch,
                    &content,
                    opts.push,
                    opts.is_tty,
                ));
            }
            Ok("edit_title") => {
                let edited: Result<String, _> =
                    cliclack::input("Edit PR title:").default_input(&title).interact();
                match edited {
                    Ok(new_title) if !new_title.is_empty() => {
                        title = new_title;
                    }
                    Ok(_) => {
                        cliclack::log::warning("Empty title — keeping current.").ok();
                    }
                    Err(_) => {
                        cliclack::log::info("Cancelled.").ok();
                    }
                }
                continue;
            }
            Ok("edit_body") => {
                match edit_in_editor(&body, ".md") {
                    Ok(Some(new_body)) => {
                        body = new_body;
                    }
                    Ok(None) => {
                        cliclack::log::warning("Empty body — keeping current.").ok();
                    }
                    Err(e) => {
                        cliclack::log::warning(format!("Editor failed: {e}")).ok();
                    }
                }
                continue;
            }
            Ok("context") => {
                if let Some(new_contents) = handle_context_regeneration(
                    branch,
                    target,
                    diff_result,
                    commit_log,
                    context,
                    opts.count,
                    config,
                    opts.is_tty,
                    &opts.provider_name,
                    opts.model.clone(),
                ) {
                    *contents = new_contents;
                }
                return ConfirmationResult::BackToCarousel;
            }
            Ok("abort") | Err(_) => {
                cliclack::log::info("Aborted.").ok();
                return ConfirmationResult::Exit(130);
            }
            _ => continue,
        }
    }
}

fn do_update_pr(
    git: &GitAdapter,
    platform: &dyn PlatformAdapter,
    branch: &str,
    content: &PrContent,
    push: bool,
    is_tty: bool,
) -> i32 {
    // Push first if requested
    if push {
        let sp = if is_tty {
            let s = cliclack::spinner();
            s.start("Pushing to remote...");
            Some(s)
        } else {
            None
        };

        if let Err(e) = git.push() {
            if let Some(s) = sp {
                s.stop("Failed");
            }
            cliclack::log::error(format!("Push failed: {e}")).ok();
            return 1;
        }

        if let Some(s) = sp {
            s.stop("Pushed");
        }
    }

    // Update the PR
    let label = platform.pr_label();
    let sp = if is_tty {
        let s = cliclack::spinner();
        s.start(format!("Updating {label}..."));
        Some(s)
    } else {
        None
    };

    match platform.update_pr(branch, &content.title, &content.body) {
        Ok(pr_info) => {
            if let Some(s) = sp {
                s.stop("Done");
            }
            cliclack::log::success(format!("{label} updated: {}", pr_info.url)).ok();
            0
        }
        Err(e) => {
            if let Some(s) = sp {
                s.stop("Failed");
            }
            cliclack::log::error(format!("Failed to update {label}: {e}")).ok();
            1
        }
    }
}

/// Prompt for additional context, append to existing, and regenerate PR contents.
#[allow(clippy::too_many_arguments)]
fn handle_context_regeneration(
    branch: &str,
    target: &str,
    diff_result: &DiffResult,
    commit_log: &str,
    context: &mut Option<String>,
    count: usize,
    config: &ResolvedConfig,
    is_tty: bool,
    provider_name: &str,
    model: Option<String>,
) -> Option<Vec<PrContent>> {
    let input: Result<String, _> = cliclack::input("Add context for regeneration:")
        .placeholder("e.g. this PR fixes the auth redirect bug")
        .interact();

    let new_context = match input {
        Ok(ctx) if !ctx.is_empty() => ctx,
        Ok(_) => {
            cliclack::log::warning("No context provided — keeping current content.").ok();
            return None;
        }
        Err(_) => {
            cliclack::log::info("Cancelled.").ok();
            return None;
        }
    };

    // Append to existing context
    *context = Some(match context {
        Some(existing) => format!("{existing}\n{new_context}"),
        None => new_context,
    });

    match generate_pr_contents(
        branch,
        target,
        diff_result,
        commit_log,
        context.as_deref(),
        count,
        config,
        is_tty,
        provider_name,
        model,
    ) {
        Ok(new_contents) => {
            cliclack::log::success("Content regenerated with new context.").ok();
            Some(new_contents)
        }
        Err(_) => {
            cliclack::log::warning("Regeneration failed — keeping current content.").ok();
            None
        }
    }
}

fn load_context(
    context: &Option<String>,
    context_file: &Option<std::path::PathBuf>,
) -> Option<String> {
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

fn render_dry_run(contents: &[PrContent], format: OutputFormat, is_tty: bool) -> i32 {
    match format {
        OutputFormat::Json => {
            let payload: Vec<serde_json::Value> = contents
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "title": c.title,
                        "body": c.body,
                    })
                })
                .collect();
            let json = if is_tty {
                serde_json::to_string_pretty(&payload)
            } else {
                serde_json::to_string(&payload)
            }
            .unwrap_or_else(|_| "[]".to_string());
            println!("{json}");
        }
        _ => {
            for (i, content) in contents.iter().enumerate() {
                if i > 0 {
                    println!("---");
                }
                println!("Title: {}", content.title);
                println!();
                println!("{}", content.body);
            }
        }
    }
    0
}

fn create_pr(
    platform: &dyn PlatformAdapter,
    git: &GitAdapter,
    content: &PrContent,
    target: &str,
    draft: bool,
    push: bool,
    is_tty: bool,
) -> i32 {
    let label = platform.pr_label();
    let prefix = platform.pr_prefix();

    // Create the PR
    let sp = if is_tty {
        let s = cliclack::spinner();
        s.start(format!("Creating {label}..."));
        Some(s)
    } else {
        None
    };

    let request = CreatePrRequest {
        title: content.title.clone(),
        body: content.body.clone(),
        base: target.to_string(),
        draft,
        push,
    };

    match platform.create_pr(&request) {
        Ok(pr_info) => {
            if let Some(s) = sp {
                s.stop("Done");
            }
            cliclack::log::success(format!(
                "{label} {prefix}{} created: {}",
                pr_info.number, pr_info.url
            ))
            .ok();

            // Check for unpushed commits and show hint
            if !push && let Ok(true) = git.has_unpushed_commits() {
                cliclack::log::info(
                    "You have unpushed commits. Use `sk pr --push` to push and create/update the PR.",
                )
                .ok();
            }

            0
        }
        Err(e) => {
            if let Some(s) = sp {
                s.stop("Failed");
            }
            cliclack::log::error(format!("Failed to create {label}: {e}")).ok();
            1
        }
    }
}
