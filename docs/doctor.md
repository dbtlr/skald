# Doctor

The `sk doctor` command validates your environment, configuration, provider connectivity, and maintenance state. It helps diagnose issues and can auto-fix common problems.

## Usage

```sh
sk doctor              # Run all checks
sk doctor --fix        # Auto-fix all fixable issues
sk doctor --format json # Output results as JSON
```

## Check Categories

### Environment

| Check | Pass | Fail/Warn |
|-------|------|-----------|
| `git` | git is installed and in PATH | git is missing |
| `git_repo` | Inside a git repository | Not inside a git repo (warn) |
| `gh` | GitHub CLI is installed | gh is missing (warn) |

### Configuration

| Check | Pass | Fail/Warn | Fixable |
|-------|------|-----------|---------|
| `config_dir` | Config directory exists | Directory missing | Yes |
| `config_file` | Valid YAML config found | Missing or invalid | Yes (creates default) |
| `project_config` | `.skaldrc.yaml` found or absent | Parse error (warn) | No |

### Provider

| Check | Pass | Fail |
|-------|------|------|
| `claude_cli` | Claude CLI is installed | Not found in PATH |

### Maintenance

| Check | Pass | Warn | Fixable |
|-------|------|------|---------|
| `log_dir` | Log directory exists | Directory missing | Yes |
| `stale_logs` | No stale log files | Files older than 14 days | Yes (prunes) |

## Check Statuses

| Symbol | Status | Meaning |
|--------|--------|---------|
| ✓ | Pass | Check passed |
| ▲ | Warn | Non-blocking issue |
| ✗ | Fail | Blocking issue |
| ⚡ | Fixed | Issue was auto-fixed |

When a check has a suggestion, it appears on the next line prefixed with `→`.

## `--fix` Behavior

When `--fix` is passed, doctor attempts to auto-fix all fixable issues in a single pass:

- Creates missing config directory
- Creates a default config file (if directory exists)
- Creates missing log directory
- Prunes log files older than 14 days

Fixed checks show the `⚡` symbol and include what was changed.

## `--full` Deep Checks

```sh
sk doctor --full
```

When `--full` is passed, doctor runs additional live connectivity checks that go beyond static validation:

- **Provider connectivity** — sends a minimal prompt to the configured AI provider and verifies a response is returned. This confirms the provider binary is not only installed but functional and authenticated.

Full checks are skipped by default because they require network access and may take a few seconds. Use `--full` when diagnosing provider issues or verifying a fresh install.

`--full` can be combined with other flags:

```sh
sk doctor --full --fix           # Deep checks + auto-fix
sk doctor --full --format json   # Machine-readable deep check results
```

## `--format json` Output

JSON output includes a `checks` array and a `summary` object:

```json
{
  "checks": [
    {
      "category": "environment",
      "name": "git",
      "status": "pass",
      "detail": "git version 2.47.0",
      "suggestion": null,
      "was": null
    }
  ],
  "summary": {
    "pass": 7,
    "warn": 0,
    "fail": 0,
    "fixed": 0
  }
}
```

In a TTY, JSON is pretty-printed. When piped, it is compact (single-line).

## Exit Codes

- `0` — All checks passed (or only warnings/fixes)
- `1` — One or more checks failed
