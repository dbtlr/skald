# Skald (`sk`)

AI-powered git workflow CLI. Generates commit messages, PR titles, and PR descriptions using AI so you can stay in flow.

## Features

- **Smart commits** — AI-generated commit messages from your staged diff
- **Interactive carousel** — cycle through multiple message candidates before committing
- **Commit body** — generate a multi-line description alongside the title with `--body`
- **PR generation** — AI-generated PR titles and descriptions from your branch diff and commit history
- **Context injection** — pass hints via `--context`, `--context-file`, or interactively
- **Prompt templates** — 4 built-in Tera templates, fully customizable with `sk config eject`
- **Config & aliases** — layered YAML config (global + project), composable flag shortcuts
- **Doctor** — environment, config, provider, and maintenance checks with `--fix` and `--offline`
- **Multi-provider** — Claude, Codex, Gemini, OpenCode, and Copilot CLI support
- **Multi-platform** — GitHub and GitLab support with `sk pr` / `sk mr`
- **Shell completions** — bash, zsh, and fish

## Install

### Install script (recommended)

```sh
curl -fsSL https://raw.githubusercontent.com/dbtlr/skald/main/scripts/install.sh | bash
```

### From crates.io

```sh
cargo install skald-cli
```

### GitHub Releases

Pre-built binaries for Linux, macOS, and Windows are available on the [Releases page](https://github.com/dbtlr/skald/releases). Download the archive for your platform and extract the `sk` binary.

## Quick Start

```sh
# Generate a commit message interactively (carousel of candidates)
sk commit

# Auto-accept the top candidate — no prompts
sk commit -y

# Include a multi-line body in the commit
sk commit -y --body

# Preview generated messages without committing
sk commit --dry-run

# Give the AI extra context about your change
sk commit -y --context "refactored auth to use JWT"

# Preview a PR title and description without creating anything
sk pr --dry-run

# Auto-create a PR on GitHub (or update if one exists)
sk pr -y

# Push the branch and open a draft PR in one step
sk pr -y -d --push

# Validate your environment (includes provider connectivity)
sk doctor

# Skip network checks
sk doctor --offline

# Check for updates and self-upgrade
sk upgrade

# See what would happen without actually upgrading
sk upgrade --dry-run
```

## Shell Completions

```sh
sk completions zsh  >> ~/.zfunc/_sk
sk completions bash >> ~/.local/share/bash-completion/completions/sk
sk completions fish >  ~/.config/fish/completions/sk.fish
```

## Configuration

Skald uses layered YAML config: global (`~/.config/skald/config.yaml`) merged with project (`.skaldrc.yaml`). Initialize with `sk config init`.

```yaml
provider: claude
language: English
commit:
  num_candidates: 3
aliases:
  ci: "commit -n 5"
  ca: "commit -y -a"
  fix: "commit -y -a --context 'bug fix'"
```

Run `sk aliases` to see active aliases and their sources.

## Documentation

- [Getting Started](docs/getting-started.md) — install, first run, basic usage
- [CLI Reference](docs/cli-reference.md) — commands, flags, output formats
- [Configuration](docs/configuration.md) — config files, schema, merge rules
- [Aliases](docs/aliases.md) — composable flag shortcuts
- [Prompts](docs/prompts.md) — template system, customization, eject workflow
- [Commit](docs/commit.md) — commit message generation, modes, and options
- [PR](docs/pr.md) — PR title/description generation and creation
- [Platforms](docs/platforms.md) — GitHub setup and platform configuration
- [Providers](docs/providers.md) — supported AI providers, configuration, and CLI overrides
- [Doctor](docs/doctor.md) — environment validation, auto-fix, diagnostics
- [Integrations](docs/integrations.md) — worktrunk, lazygit, fugitive, git hooks

## License

MIT — see [LICENSE](LICENSE) for details.

---

Built by [Drew Butler](https://github.com/dbtlr).
