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
fn commit_stub_shows_message() {
    sk().arg("commit").assert().success().stderr(predicate::str::contains("Not yet implemented"));
}

#[test]
fn pr_stub_shows_message() {
    sk().arg("pr").assert().success().stderr(predicate::str::contains("Not yet implemented"));
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
    sk().args(["config", "init"]).env("XDG_CONFIG_HOME", tmp.path()).assert().success();
    assert!(tmp.path().join("skald/config.yaml").exists());
}

#[test]
fn config_init_existing_shows_info() {
    let tmp = tempfile::tempdir().unwrap();
    let config_dir = tmp.path().join("skald");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.yaml"), "provider: test\n").unwrap();

    sk().args(["config", "init"])
        .env("XDG_CONFIG_HOME", tmp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn aliases_exits_zero() {
    sk().arg("aliases").assert().success();
}

#[test]
fn aliases_source_exits_zero() {
    sk().args(["aliases", "--source"]).assert().success();
}

#[test]
fn doctor_stub_shows_message() {
    sk().arg("doctor").assert().success().stderr(predicate::str::contains("Not yet implemented"));
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
