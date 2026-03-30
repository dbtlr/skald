use assert_cmd::Command;
use predicates::prelude::*;

fn sk() -> Command {
    Command::cargo_bin("sk").unwrap()
}

#[test]
fn help_exits_zero() {
    sk().arg("--help").assert().success();
}

#[test]
fn version_prints_version() {
    sk().arg("--version").assert().success().stdout(predicate::str::contains("sk"));
}

#[test]
fn no_args_shows_help() {
    sk().assert().failure().stderr(predicate::str::contains("Usage"));
}

#[test]
fn completions_zsh_outputs_script() {
    sk().args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef sk"));
}

#[test]
fn completions_bash_outputs_script() {
    sk().args(["completions", "bash"]).assert().success().stdout(predicate::str::is_empty().not());
}

#[test]
fn completions_fish_outputs_script() {
    sk().args(["completions", "fish"]).assert().success().stdout(predicate::str::is_empty().not());
}

#[test]
fn commit_bare_not_in_repo_errors() {
    let tmp = tempfile::tempdir().unwrap();
    sk().arg("commit")
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not in a git repository"));
}

#[test]
fn pr_help_shows_flags() {
    sk().args(["pr", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--auto"))
        .stdout(predicate::str::contains("--title-only"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--draft"))
        .stdout(predicate::str::contains("--push"))
        .stdout(predicate::str::contains("--update"))
        .stdout(predicate::str::contains("--base"))
        .stdout(predicate::str::contains("--context"));
}

#[test]
fn pr_update_not_in_repo_errors() {
    let tmp = tempfile::tempdir().unwrap();
    sk().args(["pr", "--update"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not in a git repository"));
}

#[test]
fn pr_update_auto_not_in_repo_errors() {
    let tmp = tempfile::tempdir().unwrap();
    sk().args(["pr", "--update", "--auto"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not in a git repository"));
}

#[test]
fn pr_update_no_existing_pr_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        std::process::Command::new("git").args(args).current_dir(tmp.path()).output().unwrap()
    };

    run(&["init"]);
    run(&["config", "user.email", "test@test.com"]);
    run(&["config", "user.name", "Test"]);
    std::fs::write(tmp.path().join("file.txt"), "initial").unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "init"]);

    let output = sk().args(["pr", "--update", "--auto"]).current_dir(tmp.path()).output().unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No PR found")
            || stderr.contains("Could not detect platform")
            || stderr.contains("Failed to get remote URL")
            || stderr.contains("No open PR")
            // sandbox/CI: tracing-appender cannot write log files
            || stderr.contains("initializing rolling file appender failed"),
        "Unexpected stderr: {stderr}"
    );
}

#[test]
fn pr_not_in_repo_errors() {
    let tmp = tempfile::tempdir().unwrap();
    sk().args(["pr", "--auto"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not in a git repository"));
}

#[test]
fn mr_help_shows_flags() {
    sk().args(["mr", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--auto"))
        .stdout(predicate::str::contains("--title-only"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--draft"))
        .stdout(predicate::str::contains("--push"))
        .stdout(predicate::str::contains("--update"))
        .stdout(predicate::str::contains("--base"))
        .stdout(predicate::str::contains("--context"));
}

#[test]
fn mr_not_in_repo_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let output = sk().args(["mr", "--auto"]).current_dir(tmp.path()).output().unwrap();

    assert!(!output.status.success(), "expected failure");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Not in a git repository")
            // sandbox/CI: tracing-appender cannot write log files
            || stderr.contains("initializing rolling file appender failed"),
        "Unexpected stderr: {stderr}"
    );
}

#[test]
fn pr_no_commits_ahead_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        std::process::Command::new("git").args(args).current_dir(tmp.path()).output().unwrap()
    };

    run(&["init"]);
    run(&["config", "user.email", "test@test.com"]);
    run(&["config", "user.name", "Test"]);
    std::fs::write(tmp.path().join("file.txt"), "initial").unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "init"]);

    sk().args(["pr", "--auto", "--base", "HEAD"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("No commits found"));
}

#[test]
fn config_runs() {
    // `sk config` defaults to `sk config show`
    sk().arg("config").assert().success();
}

#[test]
fn config_show_runs() {
    sk().args(["config", "show"]).assert().success();
}

#[test]
fn config_plain_format() {
    sk().args(["config", "show", "--format", "plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("provider"));
}

#[test]
fn config_json_format() {
    sk().args(["config", "show", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("["));
}

#[test]
fn config_init_creates_file() {
    let tmp = tempfile::tempdir().unwrap();
    sk().args(["config", "init", "--provider", "claude"])
        .env("XDG_CONFIG_HOME", tmp.path())
        .assert()
        .success();
    assert!(tmp.path().join("skald/config.yaml").exists());
}

#[test]
fn config_init_existing_shows_info() {
    let tmp = tempfile::tempdir().unwrap();
    let config_dir = tmp.path().join("skald");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.yaml"), "provider: test\n").unwrap();

    sk().args(["config", "init", "--provider", "claude"])
        .env("XDG_CONFIG_HOME", tmp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn alias_exits_zero() {
    sk().arg("alias").assert().success();
}

#[test]
fn alias_source_exits_zero() {
    sk().args(["alias", "--source"]).assert().success();
}

#[test]
fn aliases_backward_compat() {
    sk().arg("aliases").assert().success();
}

#[test]
fn doctor_runs() {
    // Doctor may exit 0 or 1 depending on environment, but should not panic
    sk().arg("doctor").assert().code(predicate::in_iter([0, 1]));
}

#[test]
fn doctor_json_format() {
    let output = sk()
        .args(["doctor", "--format", "json"])
        .output()
        .expect("failed to run sk doctor --format json");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON output");
    assert!(parsed["checks"].is_array(), "expected 'checks' array in JSON output");
    assert!(parsed["summary"].is_object(), "expected 'summary' object in JSON output");
}

#[test]
fn doctor_fix_flag() {
    let tmp = tempfile::tempdir().unwrap();
    // Run with --fix in a temp config home so config_dir gets created
    sk().args(["doctor", "--fix"])
        .env("XDG_CONFIG_HOME", tmp.path())
        .assert()
        .code(predicate::in_iter([0, 1]));
    assert!(tmp.path().join("skald").exists(), "expected config dir to be created by --fix");
}

#[test]
fn unknown_command_shows_error() {
    sk().arg("nonexistent")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn verbose_flag_accepted() {
    sk().args(["-v", "config"]).assert().success();
}

#[test]
fn quiet_flag_accepted() {
    sk().args(["-q", "config"]).assert().success();
}

#[test]
fn no_color_flag_accepted() {
    sk().args(["--no-color", "config"]).assert().success();
}

#[test]
fn config_eject_creates_files() {
    let tmp = tempfile::tempdir().unwrap();
    sk().args(["config", "eject"]).env("XDG_CONFIG_HOME", tmp.path()).assert().success();
    assert!(tmp.path().join("skald/prompts/commit-title.md").exists());
    assert!(tmp.path().join("skald/prompts/system.md").exists());
}

#[test]
fn config_eject_single_template() {
    let tmp = tempfile::tempdir().unwrap();
    sk().args(["config", "eject", "commit-title"])
        .env("XDG_CONFIG_HOME", tmp.path())
        .assert()
        .success();
    assert!(tmp.path().join("skald/prompts/commit-title.md").exists());
    assert!(!tmp.path().join("skald/prompts/system.md").exists());
}

#[test]
fn config_eject_project_flag() {
    let tmp = tempfile::tempdir().unwrap();
    sk().args(["config", "eject", "--project"]).current_dir(tmp.path()).assert().success();
    assert!(tmp.path().join(".skald/prompts/commit-title.md").exists());
}

#[test]
fn commit_show_prompt_renders_template() {
    sk().args(["commit", "--show-prompt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("conventional commit format"));
}

#[test]
fn pr_show_prompt_renders_template() {
    sk().args(["pr", "--show-prompt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pull request"));
}

#[test]
fn config_eject_unknown_template_errors() {
    let tmp = tempfile::tempdir().unwrap();
    sk().args(["config", "eject", "nonexistent"])
        .env("XDG_CONFIG_HOME", tmp.path())
        .assert()
        .failure();
}

#[test]
fn commit_help_shows_flags() {
    sk().args(["commit", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--auto"))
        .stdout(predicate::str::contains("--message-only"))
        .stdout(predicate::str::contains("--amend"))
        .stdout(predicate::str::contains("--context"))
        .stdout(predicate::str::contains("--dry-run"));
}

#[test]
fn commit_extended_flag_in_help() {
    sk().args(["commit", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--extended"));
}

#[test]
fn doctor_full_flag_in_help() {
    sk().args(["doctor", "--help"]).assert().success().stdout(predicate::str::contains("--full"));
}

#[test]
fn upgrade_help() {
    sk().args(["upgrade", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--dry-run"));
}

#[test]
fn upgrade_dry_run_does_not_panic() {
    // May succeed (update found) or fail (network) — just verify no panic
    sk().args(["upgrade", "--dry-run"]).assert().code(predicate::in_iter([0, 1]));
}

#[test]
fn commit_provider_flag_in_help() {
    sk().args(["commit", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--provider"))
        .stdout(predicate::str::contains("--model"));
}

#[test]
fn pr_provider_flag_in_help() {
    sk().args(["pr", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--provider"))
        .stdout(predicate::str::contains("--model"));
}

#[test]
fn config_init_provider_flag_in_help() {
    sk().args(["config", "init", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--provider"))
        .stdout(predicate::str::contains("--model"));
}

// --- integrations ---

/// Set XDG_CONFIG_HOME to a writable temp path so tracing-appender can create log files
/// in sandboxed/CI environments.
fn sk_with_cfg(cfg_tmp: &tempfile::TempDir) -> assert_cmd::Command {
    let mut cmd = sk();
    cmd.env("XDG_CONFIG_HOME", cfg_tmp.path());
    cmd
}

#[test]
fn integrations_list_shows_options() {
    let cfg = tempfile::tempdir().unwrap();
    sk_with_cfg(&cfg).arg("integrations").assert().success();
}

#[test]
fn integrations_help() {
    sk().args(["integrations", "--help"]).assert().success();
}

#[test]
fn integrations_worktrunk_outputs_toml() {
    let cfg = tempfile::tempdir().unwrap();
    let output = sk_with_cfg(&cfg).args(["integrations", "worktrunk"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "expected success");
    assert!(stdout.contains("[tools.skald]"), "expected [tools.skald] in stdout, got: {stdout}");
}

#[test]
fn integrations_lazygit_outputs_yaml() {
    let cfg = tempfile::tempdir().unwrap();
    let output = sk_with_cfg(&cfg).args(["integrations", "lazygit"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "expected success");
    assert!(stdout.contains("customCommands"), "expected customCommands in stdout, got: {stdout}");
}

#[test]
fn integrations_fugitive_outputs_vim() {
    let cfg = tempfile::tempdir().unwrap();
    let output = sk_with_cfg(&cfg).args(["integrations", "fugitive"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "expected success");
    assert!(stdout.contains("nnoremap"), "expected nnoremap in stdout, got: {stdout}");
}

#[test]
fn integrations_hook_outputs_script() {
    let cfg = tempfile::tempdir().unwrap();
    let output = sk_with_cfg(&cfg).args(["integrations", "hook"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "expected success");
    assert!(stdout.contains("#!/bin/sh"), "expected #!/bin/sh in stdout, got: {stdout}");
}

#[test]
fn integrations_hook_install_not_in_repo() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = tempfile::tempdir().unwrap();
    sk_with_cfg(&cfg)
        .args(["integrations", "hook", "--install"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("no .git directory found"));
}

#[test]
fn integrations_hook_install_creates_file() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let cfg = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        std::process::Command::new("git").args(args).current_dir(tmp.path()).output().unwrap()
    };

    run(&["init"]);
    run(&["config", "user.email", "test@test.com"]);
    run(&["config", "user.name", "Test"]);

    sk_with_cfg(&cfg)
        .args(["integrations", "hook", "--install"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let hook_path = tmp.path().join(".git/hooks/prepare-commit-msg");
    assert!(hook_path.exists(), "hook file should exist");
    let perms = std::fs::metadata(&hook_path).unwrap().permissions();
    assert!(perms.mode() & 0o111 != 0, "hook file should be executable");
}

#[test]
fn integrations_hook_install_no_overwrite() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        std::process::Command::new("git").args(args).current_dir(tmp.path()).output().unwrap()
    };

    run(&["init"]);
    run(&["config", "user.email", "test@test.com"]);
    run(&["config", "user.name", "Test"]);

    // Pre-create the hook
    let hooks_dir = tmp.path().join(".git/hooks");
    std::fs::create_dir_all(&hooks_dir).unwrap();
    std::fs::write(hooks_dir.join("prepare-commit-msg"), "#!/bin/sh\n# existing\n").unwrap();

    sk_with_cfg(&cfg)
        .args(["integrations", "hook", "--install"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn integrations_hook_install_force_overwrites() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        std::process::Command::new("git").args(args).current_dir(tmp.path()).output().unwrap()
    };

    run(&["init"]);
    run(&["config", "user.email", "test@test.com"]);
    run(&["config", "user.name", "Test"]);

    // Pre-create the hook
    let hooks_dir = tmp.path().join(".git/hooks");
    std::fs::create_dir_all(&hooks_dir).unwrap();
    std::fs::write(hooks_dir.join("prepare-commit-msg"), "#!/bin/sh\n# existing\n").unwrap();

    sk_with_cfg(&cfg)
        .args(["integrations", "hook", "--install", "--force"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let hook_contents =
        std::fs::read_to_string(tmp.path().join(".git/hooks/prepare-commit-msg")).unwrap();
    assert!(hook_contents.contains("#!/bin/sh"), "hook should contain skald script");
    assert!(!hook_contents.contains("# existing"), "hook should be overwritten");
}

// --- end integrations ---

#[test]
fn commit_no_staged_changes_errors() {
    let tmp = tempfile::tempdir().unwrap();
    // Create a git repo with an initial commit but no staged changes
    std::process::Command::new("git").args(["init"]).current_dir(tmp.path()).output().unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "--allow-empty", "-m", "init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    sk().args(["commit", "--auto"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("No staged or unstaged changes"));
}
