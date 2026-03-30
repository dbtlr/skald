# Changelog

## v0.3.0 — 2026-03-29

### PR / Merge Request Command
- `sk pr` for AI-generated PR titles and structured descriptions
- `sk mr` as first-class command for GitLab merge requests
- Interactive mode: title carousel + body preview + confirmation menu
- `--auto` for non-interactive PR creation
- `--update` to regenerate and update an existing PR's title/description
- `--dry-run` and `--title-only` for previewing without side effects
- `--draft` to create draft PRs
- `--push` to push before creating/updating
- `--base` to specify target branch (defaults to config `pr_target`)
- `--context` for developer-provided context
- `$EDITOR` integration for body editing (resolves `$VISUAL` → `$EDITOR` → `vi`)
- Diff source logic: without `--push`, diffs reflect only pushed commits

### GitLab Support
- `GitLabAdapter` shelling to `glab` CLI for MR operations
- Platform auto-detection from remote URL (github.com, gitlab.com, gitlab.* subdomains)
- Config-based platform override for self-hosted instances (`platform: github` or `platform: gitlab`)
- Platform-aware terminology: "MR !42" on GitLab, "PR #42" on GitHub

### Multi-Provider Support
- Generic `CliProvider` architecture replacing hardcoded Claude CLI provider
- 5 CLI providers: Claude, Codex, Gemini, OpenCode, Copilot
- Global `--provider` and `--model` flags on all commands
- Provider and model configurable in YAML config
- Interactive `sk config init` with provider detection and selection
- Non-interactive init: `sk config init --provider claude`

### Tool Integrations
- `sk integrations` command with 4 targets
- `sk integrations worktrunk` — TOML config for worktrunk commit message generation
- `sk integrations lazygit` — YAML config for lazygit custom AI commit command
- `sk integrations fugitive` — Vim keybinding for AI commits
- `sk integrations hook` — prepare-commit-msg git hook script
- `sk integrations hook --install [--force]` — direct hook installation
- Instructions on stderr, config on stdout (pipe-friendly)

### Doctor Improvements
- Detects all 5 CLI providers (configured provider missing = failure, others = info)
- `glab` CLI availability and auth checks
- `--full` tests connectivity for configured provider

### Platform Adapter
- New `skald-platform` crate with `PlatformAdapter` trait
- `GitHubAdapter` via `gh` CLI
- `GitLabAdapter` via `glab` CLI
- `detect_platform` with config override for self-hosted instances

### Breaking Changes
- Default provider name changed from `claude-cli` to `claude`
- `config init` now requires provider selection (interactive or `--provider` flag)
- Built-in prompt templates reduced from 5 to 4 (`pr-title` and `pr-description` merged into `pr`)

## v0.2.1 — 2026-03-29

### Fixes
- Set version from git tag in release workflow

## v0.2.0 — 2026-03-29

### Upgrade Command
- `sk upgrade` — check GitHub Releases for newer version and self-replace the binary
- `sk upgrade --dry-run` — check without downloading
- Platform detection for download URL (Linux/macOS x86_64/aarch64, Windows)
- Atomic binary replacement (rename on same filesystem, copy+delete otherwise)

### Doctor
- Version update check added to maintenance checks
- Warns when a newer version is available with upgrade suggestion

### Other
- Global `-C <path>` flag to run as if started in a different directory
- Fixed config directory resolution on macOS
- Renamed `aliases` command to `alias` (with `aliases` as backward-compatible alias)
- Added tracing instrumentation to doctor checks

## v0.1.0 — 2026-03-28

Initial release.

### Commit Command
- Interactive carousel for selecting from multiple AI-generated commit messages
- `--auto` mode for non-interactive, scripting-friendly usage
- `--message-only` to print the generated message without committing
- `--extended` to generate a commit body alongside the title
- `--amend` support for amending the previous commit
- `--dry-run` to preview without side effects

### Context Injection
- `--context` flag for inline hints to the AI
- `--context-file` to load context from a file
- Interactive context prompt in TTY mode

### Config System
- YAML configuration with XDG-compliant paths
- Layered merge: project (`.skaldrc.yaml`) overrides global config
- `sk config init` to scaffold a default config
- `sk config show` with plain, table, and JSON output formats

### Alias System
- Composable flag shortcuts defined in config
- Project aliases override global aliases
- `sk aliases` to list active aliases with sources
- `sk aliases --source` to show where each alias is defined

### Prompt Templates
- 5 built-in templates (system, commit-title, commit-body, pr-title, pr-body)
- Tera template engine with diff, context, and language variables
- `sk config eject` to export templates for customization
- `--show-prompt` to preview the rendered prompt without calling the provider
- Project-scoped templates via `--project` flag on eject

### Doctor
- Environment checks (git, gh CLI)
- Configuration checks (config dir, config file, project config)
- Provider checks (Claude CLI availability)
- Maintenance checks (log dir, stale log files)
- `--fix` for auto-remediation of fixable issues
- `--full` for live provider connectivity testing
- `--format json` for machine-readable output

### Shell Completions
- bash, zsh, and fish completions via `sk completions <shell>`

### Git VCS Adapter
- Staged and unstaged diff extraction
- Diff filtering (binary files, lock files)
- Repository state detection

### Claude Code CLI Provider
- AI provider backed by the Claude Code CLI
- Structured prompt construction from templates

### Logging
- Daily log rotation
- Verbosity levels (`-v`, `-vv`, `-vvv`)
- Quiet mode (`-q`)

### Output
- Plain, table, and JSON output formats
- `NO_COLOR` and `--no-color` support
- TTY detection for interactive vs piped behavior

### CI/CD
- GitHub Actions CI for Linux, macOS, and Windows
- Cross-platform release builds (x86_64 + aarch64)
- Install script (`scripts/install.sh`) for curl-based installation
