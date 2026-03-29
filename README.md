# Skald (`sk`)

AI-powered git workflow CLI. Generates commit messages, PR titles, and PR descriptions using AI so you can stay in flow.

## Features

- **Smart commits** — AI-generated commit messages from your staged diff
- **Interactive carousel** — cycle through multiple message candidates before committing
- **Extended descriptions** — generate a commit body alongside the title
- **PR generation** — AI-generated PR titles and descriptions from your branch diff and commit history
- **Context injection** — pass hints via `--context`, `--context-file`, or interactively
- **Prompt templates** — 5 built-in Tera templates, fully customizable with `sk config eject`
- **Config & aliases** — layered YAML config (global + project), composable flag shortcuts
- **Doctor** — environment, config, provider, and maintenance checks with `--fix` and `--full`
- **Multi-platform** — GitHub and GitLab support with `sk pr` / `sk mr`
- **Shell completions** — bash, zsh, and fish

## Install

### From source

```sh
cargo install --path crates/skald
```

### Install script

```sh
curl -fsSL https://raw.githubusercontent.com/dbtlr/skald/main/scripts/install.sh | bash
```

### GitHub Releases

Pre-built binaries for Linux, macOS, and Windows are available on the [Releases page](https://github.com/dbtlr/skald/releases).

## Quick Start

```sh
# Generate a commit message interactively (carousel of candidates)
sk commit

# Auto-accept the top candidate — no prompts
sk commit --auto

# Include an extended description in the commit body
sk commit --auto --extended

# Print just the message, don't commit
sk commit --message-only

# Give the AI extra context about your change
sk commit --auto --context "refactored auth to use JWT"

# Preview a PR title and description without creating anything
sk pr --dry-run

# Get multiple PR title suggestions
sk pr --title-only

# Auto-create a PR on GitHub
sk pr --auto

# Push the branch and open a draft PR in one step
sk pr --auto --draft --push

# Validate your environment
sk doctor

# Run full checks including live provider connectivity
sk doctor --full

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
provider: claude-cli
language: English
commit:
  num_candidates: 3
  extended: false

aliases:
  ci: "commit -n 5"
  ca: "commit --auto -A"
  fix: "commit --auto -a --context 'bug fix'"
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
- [Doctor](docs/doctor.md) — environment validation, auto-fix, diagnostics

## License

MIT — see [LICENSE](LICENSE) for details.

---

Built by [Drew Butler](https://github.com/dbtlr).
