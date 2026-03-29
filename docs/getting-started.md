# Getting Started

## Install

### From source (cargo)

```sh
cargo install --path crates/skald
```

This installs the `sk` binary to your Cargo bin directory.

### Verify

```sh
sk --version
```

## First Run

Run `sk --help` to see all available commands:

```sh
sk --help
```

## Shell Completions

Generate completions for your shell and add them to your shell config:

```sh
# zsh
sk completions zsh > ~/.zfunc/_sk

# bash
sk completions bash > ~/.local/share/bash-completion/completions/sk

# fish
sk completions fish > ~/.config/fish/completions/sk.fish
```

## Commit Workflow

Stage your changes and generate a commit message:

```sh
# Interactive carousel — pick from multiple suggestions
sk commit

# Auto-accept the top suggestion, no prompts
sk commit --auto

# Stage all modified files and auto-commit
sk commit --auto -a
```

## PR Workflow

Generate a PR title and description from your branch:

```sh
# Preview what would be created (no side effects)
sk pr --dry-run

# Generate title suggestions only
sk pr --title-only

# Auto-create a PR on GitHub
sk pr --auto

# Push the branch and open a draft PR in one step
sk pr --auto --draft --push
```

`sk pr --auto` requires the GitHub CLI (`gh`). See [platforms.md](platforms.md) for setup.

## What's Next

- [Commit](commit.md) — full commit workflow reference
- [PR](pr.md) — PR generation and creation options
- [Configuration](configuration.md) — layered config, project overrides, aliases
- [Doctor](doctor.md) — validate your environment
