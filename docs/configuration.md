# Configuration

Skald uses YAML configuration files with a layered merge strategy.

## Config File Locations

| Scope   | Path                                        |
|---------|---------------------------------------------|
| Global  | `~/.config/skald/config.yaml` (Linux/macOS) |
| Global  | `%APPDATA%\skald\config.yaml` (Windows)     |
| Project | `.skaldrc.yaml` (discovered upward from cwd) |

The global config path respects `XDG_CONFIG_HOME` on Linux.

## Creating a Config

```sh
sk config init
```

Creates a global config file with commented defaults. If the file already exists, it reports the path without overwriting.

## Viewing Config

```sh
sk config show              # table format (default in TTY)
sk config show --format plain
sk config show --format json
```

Shows all resolved values with their sources (`default`, `global`, or `project`).

## Config Schema

```yaml
# AI provider (claude, codex, gemini, opencode, copilot)
provider: claude

# Language for generated messages
language: English

# Default PR target branch
pr_target: main

# VCS platform (github, gitlab, etc.)
platform: github

# Version control system
vcs: git

# Provider-specific settings
providers:
  claude:
    model: claude-sonnet-4-20250514
  codex:
    model: gpt-4o
  gemini:
    model: gemini-2.5-flash

# Aliases (see docs/aliases.md)
aliases:
  ci: "commit -n 5"
```

All fields are optional. Unset fields use built-in defaults.

## Merge Rules

Resolution follows a specificity chain:

```
CLI flag > project config > global config > built-in default
```

For scalar fields (`provider`, `language`, etc.), the most-specific value wins entirely.

For map fields (`providers`, `aliases`), entries are merged key-by-key. A project alias with the same name as a global alias replaces it; other global aliases are preserved.

## Environment Variable Expansion

String values can reference environment variables with `$VAR` syntax:

```yaml
providers:
  anthropic-api:
    api_key: $ANTHROPIC_API_KEY
```

Variables are expanded at config load time. A missing variable produces a clear error with the variable name and the config key that referenced it.

## Example: Minimal Global Config

```yaml
provider: claude
language: English
```

## Example: Project Override

`.skaldrc.yaml` in a project root:

```yaml
pr_target: develop
language: Spanish

aliases:
  ci: "commit -n 10 --context 'enterprise project'"
```
