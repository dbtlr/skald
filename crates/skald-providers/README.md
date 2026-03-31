# skald-providers

AI provider trait and implementations for [skald](https://github.com/dbtlr/skald).

> **Note:** This is an internal implementation crate for skald. It is not intended for direct external use and its API may change without notice.

## What it provides

- **Provider trait** — async interface for AI-powered commit message and PR description generation
- **CLI provider** — generic implementation that shells out to AI CLI tools (Claude, Codex, Gemini, OpenCode, Copilot)
- **Provider config** — model resolution, availability detection, and per-provider configuration
- **Model catalog** — curated model lists with runtime fetch and local cache

For more information, see the [skald repository](https://github.com/dbtlr/skald).

## License

MIT
