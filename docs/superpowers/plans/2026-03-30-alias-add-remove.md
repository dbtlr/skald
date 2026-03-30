# Alias Add/Remove Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `add` and `remove` subcommands to `sk alias` so aliases can be managed from the CLI without manually editing YAML.

**Architecture:** Restructure the `Alias` command from a flat struct to a subcommand enum (`List`, `Add`, `Remove`). Add a new `config/writer.rs` module in `skald-core` for the read-mutate-validate-write cycle. Two new error variants for alias-specific write failures.

**Tech Stack:** Rust, clap (subcommands), serde_yaml_ng, tempfile (tests)

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `crates/skald-core/src/config/writer.rs` | Create | `add_alias()` and `remove_alias()` — YAML read/mutate/validate/write |
| `crates/skald-core/src/config/mod.rs` | Modify | Add `pub mod writer;` and re-exports |
| `crates/skald-core/src/error/mod.rs` | Modify | Add `AliasAlreadyExists` and `AliasNotFound` error variants |
| `crates/skald/src/cli/root.rs` | Modify | Restructure `Alias` to subcommand enum with `List`, `Add`, `Remove` |
| `crates/skald/src/cli/aliases.rs` | Modify | Handle new subcommands, delegate writes to `skald-core` |
| `crates/skald/src/main.rs` | Modify | Update `Command::Alias` match arm for new subcommand structure |
| `docs/aliases.md` | Modify | Document `add` and `remove` subcommands |

---

### Task 1: Add Error Variants

**Files:**
- Modify: `crates/skald-core/src/error/mod.rs`

- [ ] **Step 1: Add `AliasAlreadyExists` and `AliasNotFound` variants**

In `crates/skald-core/src/error/mod.rs`, add these variants to the `SkaldError` enum after the existing alias errors:

```rust
#[error("Alias '{name}' already exists (expands to \"{expansion}\"). Use --force to overwrite.")]
AliasAlreadyExists { name: String, expansion: String },

#[error("Alias '{name}' not found in {scope} config.")]
AliasNotFound { name: String, scope: String },
```

- [ ] **Step 2: Add suggestions for the new variants**

In the `suggestion()` method, add:

```rust
Self::AliasAlreadyExists { .. } => {
    Some("Use --force to replace the existing alias.")
}
Self::AliasNotFound { .. } => {
    Some("Run `sk alias list` to see active aliases.")
}
```

- [ ] **Step 3: Add exit codes for the new variants**

In the `exit_code()` method, add `Self::AliasAlreadyExists { .. }` and `Self::AliasNotFound { .. }` to the existing `=> 1` arm.

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p skald-core`
Expected: compiles with no errors (warnings about unused variants are fine)

- [ ] **Step 5: Commit**

```bash
git add crates/skald-core/src/error/mod.rs
git commit -m "feat(core): add AliasAlreadyExists and AliasNotFound error variants"
```

---

### Task 2: Create `config/writer.rs` with Tests

**Files:**
- Create: `crates/skald-core/src/config/writer.rs`
- Modify: `crates/skald-core/src/config/mod.rs`

- [ ] **Step 1: Write failing tests for `add_alias`**

Create `crates/skald-core/src/config/writer.rs` with tests only:

```rust
use std::path::Path;

use crate::config::aliases::validate_aliases;
use crate::config::loader::load_file;
use crate::config::schema::RawConfig;
use crate::error::{Result, SkaldError};

/// Add an alias to a YAML config file.
///
/// Creates the file if it doesn't exist. Fails if the alias already exists
/// unless `force` is true. Validates the full alias set before writing.
pub fn add_alias(name: &str, expansion: &str, path: &Path, force: bool) -> Result<()> {
    todo!()
}

/// Remove an alias from a YAML config file.
///
/// Fails if the file doesn't exist or the alias is not found.
/// `scope` is used in error messages (e.g. "global" or "project").
pub fn remove_alias(name: &str, path: &Path, scope: &str) -> Result<()> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_config(dir: &TempDir, content: &str) -> std::path::PathBuf {
        let path = dir.path().join("config.yaml");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn add_alias_to_empty_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.yaml");
        add_alias("ci", "commit -n 5", &path, false).unwrap();
        let config = load_file(&path).unwrap().unwrap();
        assert_eq!(config.aliases.unwrap()["ci"], "commit -n 5");
    }

    #[test]
    fn add_alias_to_existing_config() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "provider: claude\n");
        add_alias("ci", "commit -n 5", &path, false).unwrap();
        let config = load_file(&path).unwrap().unwrap();
        assert_eq!(config.provider.unwrap(), "claude");
        assert_eq!(config.aliases.unwrap()["ci"], "commit -n 5");
    }

    #[test]
    fn add_alias_preserves_existing_aliases() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "aliases:\n  ci: \"commit -n 5\"\n");
        add_alias("p", "pr", &path, false).unwrap();
        let config = load_file(&path).unwrap().unwrap();
        let aliases = config.aliases.unwrap();
        assert_eq!(aliases["ci"], "commit -n 5");
        assert_eq!(aliases["p"], "pr");
    }

    #[test]
    fn add_alias_fails_if_exists_without_force() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "aliases:\n  ci: \"commit -n 5\"\n");
        let err = add_alias("ci", "commit -n 10", &path, false).unwrap_err();
        assert!(matches!(err, SkaldError::AliasAlreadyExists { .. }));
    }

    #[test]
    fn add_alias_overwrites_with_force() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "aliases:\n  ci: \"commit -n 5\"\n");
        add_alias("ci", "commit -n 10", &path, true).unwrap();
        let config = load_file(&path).unwrap().unwrap();
        assert_eq!(config.aliases.unwrap()["ci"], "commit -n 10");
    }

    #[test]
    fn add_alias_validates_before_write() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.yaml");
        // "commit" shadows a builtin
        let err = add_alias("commit", "commit -n 5", &path, false).unwrap_err();
        assert!(matches!(err, SkaldError::AliasShadowsBuiltin { .. }));
        // File should not have been created
        assert!(!path.exists());
    }

    #[test]
    fn add_alias_validates_expansion_targets_builtin() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.yaml");
        let err = add_alias("bad", "nonexistent --flag", &path, false).unwrap_err();
        assert!(matches!(err, SkaldError::AliasInvalidCommand { .. }));
    }
}
```

- [ ] **Step 2: Register the module**

In `crates/skald-core/src/config/mod.rs`, add:

```rust
pub mod writer;
```

And add re-exports:

```rust
pub use writer::{add_alias, remove_alias};
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p skald-core writer`
Expected: all tests fail with `not yet implemented`

- [ ] **Step 4: Implement `add_alias`**

Replace the `todo!()` in `add_alias`:

```rust
pub fn add_alias(name: &str, expansion: &str, path: &Path, force: bool) -> Result<()> {
    let mut config = load_file(path)?.unwrap_or_default();
    let aliases = config.aliases.get_or_insert_with(Default::default);

    if !force {
        if let Some(existing) = aliases.get(name) {
            return Err(SkaldError::AliasAlreadyExists {
                name: name.to_string(),
                expansion: existing.clone(),
            });
        }
    }

    aliases.insert(name.to_string(), expansion.to_string());
    validate_aliases(aliases)?;

    let yaml = serde_yaml_ng::to_string(&config)
        .map_err(|e| SkaldError::Other { message: format!("Failed to serialize config: {e}") })?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, yaml)?;
    Ok(())
}
```

- [ ] **Step 5: Run add_alias tests**

Run: `cargo test -p skald-core writer::tests::add_alias`
Expected: all `add_alias` tests pass

- [ ] **Step 6: Add remove_alias tests**

Append to the `mod tests` block:

```rust
    #[test]
    fn remove_alias_succeeds() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "aliases:\n  ci: \"commit -n 5\"\n  p: \"pr\"\n");
        remove_alias("ci", &path, "global").unwrap();
        let config = load_file(&path).unwrap().unwrap();
        let aliases = config.aliases.unwrap();
        assert!(!aliases.contains_key("ci"));
        assert_eq!(aliases["p"], "pr");
    }

    #[test]
    fn remove_alias_not_found() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "aliases:\n  ci: \"commit -n 5\"\n");
        let err = remove_alias("nonexistent", &path, "global").unwrap_err();
        assert!(matches!(err, SkaldError::AliasNotFound { .. }));
    }

    #[test]
    fn remove_alias_no_config_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.yaml");
        let err = remove_alias("ci", &path, "global").unwrap_err();
        assert!(matches!(err, SkaldError::AliasNotFound { .. }));
    }
```

- [ ] **Step 7: Implement `remove_alias`**

Replace the `todo!()` in `remove_alias`:

```rust
pub fn remove_alias(name: &str, path: &Path, scope: &str) -> Result<()> {
    let mut config = match load_file(path)? {
        Some(c) => c,
        None => {
            return Err(SkaldError::AliasNotFound {
                name: name.to_string(),
                scope: scope.to_string(),
            });
        }
    };

    let aliases = match config.aliases.as_mut() {
        Some(a) if a.contains_key(name) => a,
        _ => {
            return Err(SkaldError::AliasNotFound {
                name: name.to_string(),
                scope: scope.to_string(),
            });
        }
    };

    aliases.remove(name);

    let yaml = serde_yaml_ng::to_string(&config)
        .map_err(|e| SkaldError::Other { message: format!("Failed to serialize config: {e}") })?;
    std::fs::write(path, yaml)?;
    Ok(())
}
```

- [ ] **Step 8: Run all writer tests**

Run: `cargo test -p skald-core writer`
Expected: all tests pass

- [ ] **Step 9: Commit**

```bash
git add crates/skald-core/src/config/writer.rs crates/skald-core/src/config/mod.rs
git commit -m "feat(core): add config writer with add_alias and remove_alias"
```

---

### Task 3: Restructure CLI `Alias` Command to Subcommands

**Files:**
- Modify: `crates/skald/src/cli/root.rs:158-163`

- [ ] **Step 1: Replace the `Alias` variant with a subcommand enum**

In `crates/skald/src/cli/root.rs`, replace:

```rust
    /// List active aliases and their sources
    #[command(alias = "aliases")]
    Alias {
        /// Show which config file each alias comes from
        #[arg(long)]
        source: bool,
    },
```

with:

```rust
    /// Manage aliases
    #[command(alias = "aliases", arg_required_else_help = true)]
    Alias {
        #[command(subcommand)]
        action: AliasAction,
    },
```

- [ ] **Step 2: Add the `AliasAction` enum**

After the `ConfigAction` enum, add:

```rust
#[derive(clap::Subcommand, Debug)]
pub enum AliasAction {
    /// List all active aliases
    List {
        /// Show which config file each alias comes from
        #[arg(long)]
        source: bool,
    },
    /// Add a new alias
    Add {
        /// Alias name
        name: String,
        /// Command expansion (e.g. "commit -n 5")
        expansion: String,
        /// Write to project config (.skaldrc.yaml) instead of global
        #[arg(long)]
        project: bool,
        /// Overwrite an existing alias
        #[arg(long)]
        force: bool,
    },
    /// Remove an alias
    Remove {
        /// Alias name to remove
        name: String,
        /// Remove from project config (.skaldrc.yaml) instead of global
        #[arg(long)]
        project: bool,
    },
}
```

- [ ] **Step 3: Verify it compiles (expect errors from main.rs)**

Run: `cargo check -p skald 2>&1 | head -20`
Expected: compile errors in `main.rs` where `Command::Alias { source }` is matched — this is expected and will be fixed in Task 4.

- [ ] **Step 4: Commit**

```bash
git add crates/skald/src/cli/root.rs
git commit -m "feat(cli): restructure alias command into list/add/remove subcommands"
```

---

### Task 4: Wire Up CLI Handlers

**Files:**
- Modify: `crates/skald/src/cli/aliases.rs`
- Modify: `crates/skald/src/main.rs:218-224`

- [ ] **Step 1: Update `aliases.rs` with new handler functions**

Replace the entire contents of `crates/skald/src/cli/aliases.rs`:

```rust
use std::path::Path;

use skald_core::config::{self, ResolvedConfig};
use skald_core::output::OutputFormat;

pub fn run_list(
    config: &ResolvedConfig,
    format: OutputFormat,
    is_tty: bool,
    show_source: bool,
) -> i32 {
    if config.aliases.is_empty() {
        cliclack::log::info("No aliases configured.").ok();
        return 0;
    }

    let mut sorted: Vec<(&String, &String)> = config.aliases.iter().collect();
    sorted.sort_by_key(|(name, _)| name.as_str());

    if show_source {
        let headers = vec!["Alias", "Expansion", "Source"];
        let rows: Vec<Vec<String>> = sorted
            .iter()
            .map(|(name, expansion)| {
                let source = config
                    .sources
                    .get(format!("alias.{name}").as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "config".to_string());
                vec![(*name).clone(), (*expansion).clone(), source]
            })
            .collect();
        print!("{}", format.render_rows(&headers, &rows, is_tty));
    } else {
        let headers = vec!["Alias", "Expansion"];
        let rows: Vec<Vec<String>> = sorted
            .iter()
            .map(|(name, expansion)| vec![(*name).clone(), (*expansion).clone()])
            .collect();
        print!("{}", format.render_rows(&headers, &rows, is_tty));
    }

    0
}

pub fn run_add(name: &str, expansion: &str, project: bool, force: bool) -> i32 {
    let path = resolve_config_path(project);
    match config::add_alias(name, expansion, &path, force) {
        Ok(()) => {
            let scope = if project { "project" } else { "global" };
            cliclack::log::success(format!("Added alias '{name}' → \"{expansion}\" ({scope})"))
                .ok();
            0
        }
        Err(e) => {
            cliclack::log::error(format!("{e}")).ok();
            if let Some(hint) = e.suggestion() {
                cliclack::log::info(hint).ok();
            }
            1
        }
    }
}

pub fn run_remove(name: &str, project: bool) -> i32 {
    let path = resolve_config_path(project);
    let scope = if project { "project" } else { "global" };
    match config::remove_alias(name, &path, scope) {
        Ok(()) => {
            cliclack::log::success(format!("Removed alias '{name}' from {scope} config")).ok();
            0
        }
        Err(e) => {
            cliclack::log::error(format!("{e}")).ok();
            if let Some(hint) = e.suggestion() {
                cliclack::log::info(hint).ok();
            }
            1
        }
    }
}

fn resolve_config_path(project: bool) -> std::path::PathBuf {
    if project {
        let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
        config::discover_project_config(&cwd).unwrap_or_else(|| cwd.join(".skaldrc.yaml"))
    } else {
        config::global_config_path()
    }
}
```

- [ ] **Step 2: Update `main.rs` match arm**

In `crates/skald/src/main.rs`, replace the `Command::Alias` match arm (lines 218-224):

```rust
        Command::Alias { source } => match config_result {
            Ok(ref cfg) => cli::aliases::run_aliases(cfg, fmt, is_tty, source),
            Err(ref e) => {
                cliclack::log::error(format!("Failed to load config: {e}")).ok();
                1
            }
        },
```

with:

```rust
        Command::Alias { action } => {
            use cli::AliasAction;
            match action {
                AliasAction::List { source } => match config_result {
                    Ok(ref cfg) => cli::aliases::run_list(cfg, fmt, is_tty, source),
                    Err(ref e) => {
                        cliclack::log::error(format!("Failed to load config: {e}")).ok();
                        1
                    }
                },
                AliasAction::Add { name, expansion, project, force } => {
                    cli::aliases::run_add(&name, &expansion, project, force)
                }
                AliasAction::Remove { name, project } => {
                    cli::aliases::run_remove(&name, project)
                }
            }
        },
```

Also update the import at the top of `main.rs`. Change:

```rust
use cli::{Cli, Command, ConfigAction};
```

to:

```rust
use cli::{AliasAction, Cli, Command, ConfigAction};
```

- [ ] **Step 3: Verify it compiles and tests pass**

Run: `cargo build -p skald && cargo test -p skald-core writer`
Expected: compiles cleanly, all writer tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/skald/src/cli/aliases.rs crates/skald/src/main.rs
git commit -m "feat(cli): wire up alias add/remove/list handlers"
```

---

### Task 5: Manual Smoke Test and Fix-ups

- [ ] **Step 1: Test help output**

Run: `cargo run -- alias --help`
Expected: Shows `list`, `add`, `remove` subcommands

Run: `cargo run -- alias`
Expected: Same as `--help` (due to `arg_required_else_help`)

- [ ] **Step 2: Test add alias**

Run: `cargo run -- alias add testci "commit -n 7"`
Expected: success message, alias written to global config

Verify: `cargo run -- alias list`
Expected: `testci` appears in the list

- [ ] **Step 3: Test add duplicate fails**

Run: `cargo run -- alias add testci "commit -n 10"`
Expected: error about alias already existing, suggests `--force`

- [ ] **Step 4: Test add with force**

Run: `cargo run -- alias add testci "commit -n 10" --force`
Expected: success, overwrites the alias

- [ ] **Step 5: Test remove**

Run: `cargo run -- alias remove testci`
Expected: success message

Verify: `cargo run -- alias list`
Expected: `testci` no longer appears

- [ ] **Step 6: Test remove nonexistent**

Run: `cargo run -- alias remove nonexistent`
Expected: error message about alias not found

- [ ] **Step 7: Test validation**

Run: `cargo run -- alias add commit "commit -n 5"`
Expected: error about shadowing builtin

Run: `cargo run -- alias add bad "nonexistent --flag"`
Expected: error about invalid command

- [ ] **Step 8: Fix any issues found, then commit**

```bash
git add -A
git commit -m "fix: address issues found during alias add/remove smoke test"
```

(Skip this commit if no issues found.)

---

### Task 6: Update Documentation

**Files:**
- Modify: `docs/aliases.md`

- [ ] **Step 1: Update `docs/aliases.md`**

Replace the "Defining Aliases" and "Viewing Aliases" sections. The full updated file:

```markdown
# Aliases

Aliases are composable flag shortcuts for skald commands. They are not shell aliases -- they expand within skald before argument parsing.

## Managing Aliases

### Adding an alias

```sh
sk alias add ci "commit -n 5"             # add to global config
sk alias add ci "commit -n 5" --project   # add to project config
sk alias add ci "commit -n 10" --force    # overwrite existing alias
```

### Removing an alias

```sh
sk alias remove ci             # remove from global config
sk alias remove ci --project   # remove from project config
```

### Listing aliases

```sh
sk alias list              # list all active aliases
sk alias list --source     # include which config file each alias comes from
sk alias list --format json
```

### Manual configuration

You can also define aliases directly in your config files:

```yaml
aliases:
  ci: "commit -n 5"
  ca: "commit --auto -A"
  fix: "commit --auto -a --context 'bug fix'"
```

Each alias maps a short name to a command with flags. When you run `sk ci`, skald expands it to `sk commit -n 5` before parsing.

## How Expansion Works

1. Skald loads config before parsing CLI arguments
2. If the first argument matches an alias name, it's replaced with the expansion
3. Any additional arguments you pass are appended after the expansion
4. The expanded arguments are then parsed by clap normally

```sh
sk ci --no-extended
# expands to: sk commit -n 5 --no-extended
```

## Resolution Rules

Aliases follow the same merge rules as other config:

- **Project wins.** A project alias with the same name as a global alias replaces it.
- **Last-wins within a file.** Standard YAML duplicate-key behavior.
- Global aliases not overridden by the project are preserved.

## Restrictions

- **No shadowing builtins.** An alias cannot have the same name as a built-in command (`commit`, `pr`, `config`, `aliases`, `doctor`, `completions`).
- **No recursion.** An alias expansion cannot reference another alias as its first token.
- **Must target a builtin.** The first token of an alias expansion must be a built-in command.

Violating any of these produces a clear error when adding the alias.

## Examples

### Quick commit with more candidates

```sh
sk alias add ci "commit -n 5"
```

### Auto-commit all files

```sh
sk alias add ca "commit --auto -A"
```

### Context-aware commits

```sh
sk alias add fix "commit --auto -a --context 'bug fix'"
sk alias add feat "commit --auto -a --context 'new feature'"
```

### PR shortcut

```sh
sk alias add p "pr"
```
```

- [ ] **Step 2: Commit**

```bash
git add docs/aliases.md
git commit -m "docs: update aliases documentation with add/remove commands"
```

---

### Task 7: Run Full Test Suite

- [ ] **Step 1: Run all tests**

Run: `cargo test --workspace`
Expected: all tests pass

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: no warnings

- [ ] **Step 3: Check formatting**

Run: `cargo fmt --check`
Expected: no formatting issues

- [ ] **Step 4: Fix any issues and commit**

```bash
git add -A
git commit -m "fix: address test/lint issues"
```

(Skip if nothing to fix.)
