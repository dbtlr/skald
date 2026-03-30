# CLI Reference

## Commands

| Command | Status | Description |
|---------|--------|-------------|
| `sk commit` | Stub (M1) | Generate commit messages and commit |
| `sk pr` | Working | Generate PR title/description and create PR |
| `sk mr` | Working | Generate MR title/description (alias for pr) |
| `sk config` | Partial | View and manage configuration |
| `sk aliases` | Stub (M1) | List active aliases and their sources |
| `sk doctor` | Stub (M3) | Validate environment, config, and provider connectivity |
| `sk completions <shell>` | Working | Generate shell completions (bash, zsh, fish) |
| `sk integrations` | Working | Output integration config snippets |

## Global Flags

These flags are available on all commands:

| Flag | Short | Description |
|------|-------|-------------|
| `--verbose` | `-v` | Increase verbosity. Stackable: `-v` (info), `-vv` (debug), `-vvv` (trace) |
| `--quiet` | `-q` | Suppress all output except errors and final results |
| `--no-color` | | Disable color output. Also triggered by `NO_COLOR` env var |
| `--format` | | Output format: `plain`, `table`, `json` |
| `--provider` | | AI provider to use for this command (e.g. `claude`, `codex`, `gemini`) |
| `--model` | | Model name to pass to the provider (e.g. `claude-haiku-4-5`, `gpt-4o`) |
| `--version` | `-V` | Print version |
| `--help` | `-h` | Print help |

## Output Formats

All commands that produce structured output support three formats:

### Plain

One item per line, tab-separated values, no decoration. Default when stdout is piped.

```sh
sk config --format plain
```

### Table

Numbered rows with aligned columns. Default in TTY.

```sh
sk config --format table
```

### JSON

Structured data. Pretty-printed in TTY, compact when piped.

```sh
sk config --format json
```

## Environment Variables

| Variable | Effect |
|----------|--------|
| `NO_COLOR` | If set (any value), disables all color output |

## Logging

Logs are written to `~/.config/skald/logs/` with daily rotation and 14-day retention. Verbosity flags control what appears on stderr; the log file always captures DEBUG-level detail.
