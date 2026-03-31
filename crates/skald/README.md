# skald-cli

AI-powered git workflow CLI. Generates commit messages, PR titles, and PR descriptions using AI so you can stay in flow.

- **Smart commits** with interactive carousel or auto-accept
- **PR generation** for GitHub and GitLab
- **Multi-provider** support (Claude, Codex, Gemini, OpenCode, Copilot)
- **Customizable** via layered config, aliases, and prompt templates

## Install

```sh
cargo install skald-cli
```

Or use the install script:

```sh
curl -fsSL https://raw.githubusercontent.com/dbtlr/skald/main/scripts/install.sh | bash
```

## Usage

```sh
sk commit          # generate a commit message interactively
sk commit --auto   # auto-accept the top candidate
sk pr --auto       # create a PR with AI-generated title and description
sk doctor          # validate your environment
```

For full documentation, see the [skald repository](https://github.com/dbtlr/skald).

## License

MIT
