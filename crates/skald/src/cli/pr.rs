use skald_core::config::schema::ResolvedConfig;
use skald_core::output::OutputFormat;
use skald_core::prompts::{PromptContext, mock_prompt_context, render_prompt, resolve_template};
use skald_platform::{CreatePrRequest, detect_platform};
use skald_providers::claude_cli::ClaudeCliProvider;
use skald_providers::{PrContent, PrContext, Provider};
use skald_vcs::git::GitAdapter;
use skald_vcs::{DiffOptions, VcsAdapter};

pub struct PrOptions {
    pub show_prompt: bool,
    pub auto: bool,
    pub title_only: bool,
    pub dry_run: bool,
    pub draft: bool,
    pub push: bool,
    pub update: bool,
    pub base: Option<String>,
    pub count: usize,
    pub context: Option<String>,
    pub format: OutputFormat,
    pub is_tty: bool,
}

pub fn run_pr(opts: PrOptions, config: &ResolvedConfig) -> i32 {
    // 1. --update: not yet implemented
    if opts.update {
        cliclack::log::info("Not yet implemented \u{2014} coming in M9.").ok();
        return 0;
    }

    // 2. Detect git repo
    let git = match GitAdapter::detect() {
        Ok(g) => g,
        Err(e) => {
            cliclack::log::error(format!("Not in a git repository: {e}")).ok();
            return 1;
        }
    };

    // 3. Get current branch
    let branch = git.get_current_branch().unwrap_or_else(|_| "HEAD".to_string());

    // 4. --show-prompt: render PR template with mock context and print
    if opts.show_prompt {
        return run_show_prompt(&branch, &config.language);
    }

    // 5. Resolve target branch: --base flag -> config.pr_target -> "main"
    let target = opts
        .base
        .clone()
        .unwrap_or_else(|| config.pr_target.clone());

    // 6. Get branch diff and commit log
    let diff_result = match git.get_branch_diff(
        &target,
        &DiffOptions {
            staged: false,
            exclude_patterns: vec![],
        },
    ) {
        Ok(d) => d,
        Err(e) => {
            cliclack::log::error(format!("Failed to get branch diff: {e}")).ok();
            return 1;
        }
    };

    let commit_log = match git.get_commit_log(&target) {
        Ok(log) => log,
        Err(e) => {
            cliclack::log::error(format!("Failed to get commit log: {e}")).ok();
            return 1;
        }
    };

    // 7. If commit log is empty, error
    if commit_log.trim().is_empty() {
        cliclack::log::error(format!("No commits found ahead of '{target}'.")).ok();
        return 1;
    }

    // 8. Determine effective mode (title_only implied by -n when not auto/dry-run)
    let effective_title_only =
        opts.title_only || (opts.count != 3 && !opts.auto && !opts.dry_run);

    // 9. Build PromptContext
    let prompt_ctx = PromptContext::new()
        .set("branch", &branch)
        .set("target_branch", &target)
        .set("diff_stat", &diff_result.stat)
        .set("context", opts.context.as_deref().unwrap_or(""))
        .set("language", &config.language)
        .set("num_suggestions", &opts.count.to_string())
        .set("commit_log", &commit_log);

    // 10. Resolve and render "pr" template, prepend system message
    let template = match resolve_template("pr", None, None) {
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

    let system_template = resolve_template("system", None, None).unwrap_or_default();
    let system_msg = render_prompt(&system_template, &prompt_ctx).unwrap_or(system_template);
    let full_prompt = format!("{system_msg}\n\n{rendered_prompt}");

    // 11. Build PrContext
    let pr_ctx = PrContext {
        diff: diff_result.diff.clone(),
        commit_log: commit_log.clone(),
        target_branch: target.clone(),
        rendered_prompt: full_prompt,
        extra_context: opts.context.clone(),
    };

    // 12. Create provider, show spinner, call generate_pr_content
    let model = config
        .providers
        .get(&config.provider)
        .and_then(|p| p.model.clone());
    let provider = ClaudeCliProvider::new(model);

    let count = if opts.auto { 1 } else { opts.count };

    let sp = if opts.is_tty {
        let s = cliclack::spinner();
        s.start("Generating PR content...");
        Some(s)
    } else {
        None
    };

    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            if let Some(s) = sp {
                s.stop("Failed");
            }
            cliclack::log::error(format!("Failed to create async runtime: {e}")).ok();
            return 1;
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
            return 1;
        }
    };

    if contents.is_empty() {
        cliclack::log::error("No PR content generated.").ok();
        return 1;
    }

    // 13. title_only: render titles
    if effective_title_only {
        return render_titles(&contents, opts.format, opts.is_tty);
    }

    // 14. dry_run: render payload
    if opts.dry_run {
        return render_dry_run(&contents, opts.format, opts.is_tty);
    }

    // 15. auto: detect platform, check existing PR, create PR
    if opts.auto {
        return create_pr(
            &git, &contents[0], &target, opts.draft, opts.push, opts.is_tty,
        );
    }

    // 16. No mode flag: fall back to dry_run output with a warning
    cliclack::log::warning("Interactive PR mode not yet implemented. Showing dry-run output.")
        .ok();
    render_dry_run(&contents, opts.format, opts.is_tty)
}

fn run_show_prompt(branch: &str, language: &str) -> i32 {
    let ctx = mock_prompt_context()
        .set("branch", branch)
        .set("language", language);

    match resolve_template("pr", None, None) {
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

fn render_titles(contents: &[PrContent], format: OutputFormat, is_tty: bool) -> i32 {
    let titles: Vec<&str> = contents.iter().map(|c| c.title.as_str()).collect();
    match format {
        OutputFormat::Json => {
            let json = if is_tty {
                serde_json::to_string_pretty(&titles)
            } else {
                serde_json::to_string(&titles)
            }
            .unwrap_or_else(|_| "[]".to_string());
            println!("{json}");
        }
        _ => {
            for title in &titles {
                println!("{title}");
            }
        }
    }
    0
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
    git: &GitAdapter,
    content: &PrContent,
    target: &str,
    draft: bool,
    push: bool,
    is_tty: bool,
) -> i32 {
    // Detect platform from remote URL
    let remote_url = match git.get_remote_url() {
        Ok(url) => url,
        Err(e) => {
            cliclack::log::error(format!("Failed to get remote URL: {e}")).ok();
            return 1;
        }
    };

    let platform = match detect_platform(&remote_url) {
        Some(p) => p,
        None => {
            cliclack::log::error(
                "Could not detect platform from remote URL. Currently only GitHub is supported.",
            )
            .ok();
            return 1;
        }
    };

    let branch = git
        .get_current_branch()
        .unwrap_or_else(|_| "HEAD".to_string());

    // Check for existing PR
    match platform.pr_exists(&branch) {
        Ok(Some(existing)) => {
            cliclack::log::warning(format!(
                "PR #{} already exists for branch '{}': {}",
                existing.number, branch, existing.url
            ))
            .ok();
            return 0;
        }
        Ok(None) => {}
        Err(e) => {
            cliclack::log::error(format!("Failed to check for existing PR: {e}")).ok();
            return 1;
        }
    }

    // Create the PR
    let sp = if is_tty {
        let s = cliclack::spinner();
        s.start("Creating PR...");
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
            cliclack::log::success(format!("PR #{} created: {}", pr_info.number, pr_info.url))
                .ok();

            // Check for unpushed commits and show hint
            if !push {
                if let Ok(true) = git.has_unpushed_commits() {
                    cliclack::log::info(
                        "You have unpushed commits. Use `sk pr --push --update` to push and update the PR.",
                    )
                    .ok();
                }
            }

            0
        }
        Err(e) => {
            if let Some(s) = sp {
                s.stop("Failed");
            }
            cliclack::log::error(format!("Failed to create PR: {e}")).ok();
            1
        }
    }
}
