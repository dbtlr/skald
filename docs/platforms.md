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

### Requirements

- [GitLab CLI (`glab`)](https://gitlab.com/gitlab-org/cli) installed and on your PATH
- Authenticated via `glab auth login`

### Setup

```sh
# Install glab
brew install glab       # macOS
sudo apt install glab   # Debian/Ubuntu

# Authenticate
glab auth login
```

### Platform Detection

Skald detects GitLab automatically from your git remote URL:

- `https://gitlab.com/user/repo.git`
- `git@gitlab.com:user/repo.git`
- Self-hosted instances with `gitlab` in the hostname (e.g., `gitlab.company.com`)

For self-hosted instances without `gitlab` in the hostname, set the platform explicitly in config.

## Self-Hosted / Enterprise

For GitHub Enterprise or GitLab instances with custom domains, set the platform in your config:

```yaml
# ~/.config/skald/config.yaml or .skaldrc.yaml
platform: github   # Force GitHub platform
# or
platform: gitlab   # Force GitLab platform
```

When `platform` is set to a specific value, URL detection is skipped.

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
