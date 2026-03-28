# Skald

AI-powered git workflow CLI. Generate commit messages, PR titles, and PR descriptions.

## Install

```sh
cargo install --path crates/skald
```

## Usage

```sh
sk commit          # Generate commit message and commit
sk pr              # Generate PR title/description and create PR
sk config          # View configuration
sk aliases         # List active aliases
sk doctor          # Validate environment and config
sk completions zsh # Generate shell completions
```

## Global Flags

| Flag | Description |
|------|-------------|
| `-v` / `-vv` / `-vvv` | Increase verbosity |
| `-q` | Quiet mode — errors only |
| `--no-color` | Disable color output |
| `--format <plain\|table\|json>` | Output format |

## License

MIT
