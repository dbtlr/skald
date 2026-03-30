use crate::cli::IntegrationTarget;

const WORKTRUNK_CONFIG: &str = r#"[tools.skald]
command = "sk"
args = ["commit", "--message-only", "--auto"]
"#;

const WORKTRUNK_INSTRUCTIONS: &str = "\
Add the following to your worktrunk config.toml (usually ~/.config/worktrunk/config.toml):";

const LAZYGIT_CONFIG: &str = r#"customCommands:
  - key: "C"
    context: "files"
    command: "sk commit"
    description: "AI-generated commit message"
    subprocess: true
"#;

const LAZYGIT_INSTRUCTIONS: &str = "\
Add the following to your lazygit config.yml (usually ~/.config/lazygit/config.yml):";

const FUGITIVE_CONFIG: &str = r#"" Skald: AI-generated commit message
nnoremap <leader>sc :!sk commit --auto<CR>
"#;

const FUGITIVE_INSTRUCTIONS: &str = "\
Add the following to your .vimrc or init.vim:";

const HOOK_SCRIPT: &str = r#"#!/bin/sh
# Skald prepare-commit-msg hook
COMMIT_MSG_FILE=$1
COMMIT_SOURCE=$2
if [ -z "$COMMIT_SOURCE" ]; then
    MSG=$(sk commit --message-only --auto 2>/dev/null)
    if [ $? -eq 0 ] && [ -n "$MSG" ]; then
        echo "$MSG" > "$COMMIT_MSG_FILE"
    fi
fi
"#;

const HOOK_INSTRUCTIONS: &str = "\
Add the following to .git/hooks/prepare-commit-msg and make it executable (chmod +x):";

pub fn run_integrations(target: Option<IntegrationTarget>) -> i32 {
    match target {
        None => run_list(),
        Some(IntegrationTarget::Worktrunk) => {
            run_snippet("worktrunk", WORKTRUNK_CONFIG, WORKTRUNK_INSTRUCTIONS)
        }
        Some(IntegrationTarget::Lazygit) => {
            run_snippet("lazygit", LAZYGIT_CONFIG, LAZYGIT_INSTRUCTIONS)
        }
        Some(IntegrationTarget::Fugitive) => {
            run_snippet("fugitive", FUGITIVE_CONFIG, FUGITIVE_INSTRUCTIONS)
        }
        Some(IntegrationTarget::Hook { install, force }) => {
            if install {
                run_hook_install(force)
            } else {
                run_snippet("hook", HOOK_SCRIPT, HOOK_INSTRUCTIONS)
            }
        }
    }
}

fn run_list() -> i32 {
    eprintln!("Available integrations:");
    eprintln!("  worktrunk  — Worktrunk commit message config");
    eprintln!("  lazygit    — Lazygit custom command config");
    eprintln!("  fugitive   — Vim-fugitive keybinding config");
    eprintln!("  hook       — Git prepare-commit-msg hook");
    eprintln!();
    eprintln!("Run `sk integrations <name>` to output the config snippet.");
    eprintln!("Run `sk integrations hook --install` to install the hook directly.");
    0
}

fn run_snippet(name: &str, config: &str, instructions: &str) -> i32 {
    eprintln!("{instructions}");
    eprintln!();
    print!("{config}");
    tracing::debug!(integration = name, "output integration snippet");
    0
}

/// Resolve the actual git directory, handling both regular repos and worktrees.
/// For worktrees, .git is a file containing `gitdir: <path>`.
fn resolve_git_dir() -> Option<std::path::PathBuf> {
    use std::fs;
    let git_path = std::path::Path::new(".git");
    if !git_path.exists() {
        return None;
    }
    if git_path.is_dir() {
        return Some(git_path.to_path_buf());
    }
    // Worktree: .git is a file with "gitdir: <path>"
    let content = fs::read_to_string(git_path).ok()?;
    let git_dir = content.lines().find_map(|l| l.strip_prefix("gitdir:"))?.trim().to_string();
    // The gitdir in a worktree points to the worktree-specific dir.
    // We want the common hooks dir at the main repo, which is ../.. from the worktree gitdir.
    let worktree_git_dir = std::path::PathBuf::from(&git_dir);
    // Check if this is inside a worktrees/ subdir; if so, use common hooks
    let hooks_in_worktree = worktree_git_dir.join("hooks");
    if hooks_in_worktree.exists() || worktree_git_dir.join("config").exists() {
        return Some(worktree_git_dir);
    }
    // Fallback: try parent.parent (common git dir)
    worktree_git_dir.parent().and_then(|p| p.parent()).map(|p| p.to_path_buf())
}

pub fn run_hook_install(force: bool) -> i32 {
    use std::fs;
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    let git_dir = match resolve_git_dir() {
        Some(d) => d,
        None => {
            eprintln!("error: no .git directory found in the current directory");
            eprintln!("hint: run this command from the root of a git repository");
            return 1;
        }
    };

    let hooks_dir = git_dir.join("hooks");
    if !hooks_dir.exists()
        && let Err(e) = fs::create_dir_all(&hooks_dir)
    {
        eprintln!("error: could not create .git/hooks/: {e}");
        return 1;
    }

    let hook_path = hooks_dir.join("prepare-commit-msg");
    if hook_path.exists() && !force {
        eprintln!("error: hook already exists at {}", hook_path.display());
        eprintln!("hint: use --force to overwrite the existing hook");
        return 1;
    }

    let mut file = match fs::File::create(&hook_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: could not write hook file: {e}");
            return 1;
        }
    };

    if let Err(e) = file.write_all(HOOK_SCRIPT.as_bytes()) {
        eprintln!("error: could not write hook script: {e}");
        return 1;
    }

    #[cfg(unix)]
    {
        let mut perms = match fs::metadata(&hook_path) {
            Ok(m) => m.permissions(),
            Err(e) => {
                eprintln!("error: could not read hook file permissions: {e}");
                return 1;
            }
        };
        perms.set_mode(0o755);
        if let Err(e) = fs::set_permissions(&hook_path, perms) {
            eprintln!("error: could not set hook file permissions: {e}");
            return 1;
        }
    }

    eprintln!("Installed hook to {}", hook_path.display());
    0
}
