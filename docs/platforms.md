# Platforms

Skald detects your git platform from the remote URL and delegates PR creation to the appropriate platform CLI.

## GitHub

### Requirements

- [GitHub CLI (`gh`)](https://cli.github.com/) must be installed and authenticated.

### Setup

```sh
# Install gh
brew install gh          # macOS
sudo apt install gh      # Debian/Ubuntu

# Authenticate
gh auth login
```

### Verify

Run `sk doctor` to confirm `gh` is detected:

```sh
sk doctor
```

A passing `gh` check means the binary is found in PATH. Use `sk doctor --full` to also verify live connectivity.

### Platform Detection

Skald detects GitHub from the remote URL automatically. Both URL formats are supported:

| Format | Example |
|--------|---------|
| HTTPS | `https://github.com/owner/repo.git` |
| SSH | `git@github.com:owner/repo.git` |

No manual platform configuration is needed for GitHub.

## GitLab

GitLab support is planned for a future release (M10).

## Configuration

### Default target branch

Set `pr_target` to control which branch PRs merge into by default.

**Global config** (`~/.config/skald/config.yaml`):

```yaml
pr_target: main
```

**Project config** (`.skaldrc.yaml` in repo root):

```yaml
pr_target: develop
```

The `--base` / `-b` flag overrides both at runtime. See [configuration.md](configuration.md) for the full merge rule reference.
