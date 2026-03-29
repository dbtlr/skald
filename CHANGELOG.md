# Changelog

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
