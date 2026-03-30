# AI Providers

Skald supports multiple AI CLI tools as providers. Each provider shells out to its respective CLI binary.

## Supported Providers

| Provider | CLI Binary | Install |
|----------|-----------|---------|
| Claude | `claude` | [Claude Code](https://claude.ai/code) |
| Codex | `codex` | [Codex CLI](https://developers.openai.com/codex/cli) |
| Gemini | `gemini` | [Gemini CLI](https://github.com/google-gemini/gemini-cli) |
| OpenCode | `opencode` | [OpenCode](https://opencode.ai) |
| Copilot | `copilot` | [Copilot CLI](https://docs.github.com/copilot) |

## Configuration

Set your default provider in config:

```yaml
# ~/.config/skald/config.yaml
provider: claude
```

Set a specific model:

```yaml
providers:
  claude:
    model: claude-haiku-4-5
```

## CLI Overrides

Override the provider or model for a single command:

```sh
sk commit --provider codex
sk commit --provider gemini --model gemini-2.5-flash
sk pr --auto --provider claude
```

## Setup

Run `sk config init` to set up your provider interactively, or specify directly:

```sh
sk config init --provider claude
sk config init --provider codex --model gpt-4o
```

## Verification

```sh
sk doctor
```

Doctor checks all known providers and reports which are available.
