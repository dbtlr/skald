# AI Providers

Skald supports multiple AI providers. Providers come in two types: CLI providers (shell out to installed tools) and API providers (direct HTTP calls to AI services).

## Supported Providers

### API Providers

API providers call AI services directly over HTTP. No CLI tool installation required — just an API key.

| Provider | Service | Default Model | Auth |
|----------|---------|---------------|------|
| Anthropic | Anthropic API | `claude-sonnet-4` | API key |
| `codex` | Codex / ChatGPT backend | `gpt-5.5` | ChatGPT subscription (no API key) |

> **Renamed.** The direct ChatGPT-subscription provider was `codex-api` and is now simply `codex`. The old CLI shell-out provider (previously `codex`) is now `codex-cli`. See [Migration](#migrating-from-codex-api--codex).

### Codex via your ChatGPT subscription (`codex`)

The `codex` provider reuses the login created by the [Codex CLI](https://developers.openai.com/codex/cli) — it reads `~/.codex/auth.json` and calls the Codex inference backend directly. If you've signed into Codex with a **ChatGPT subscription**, `sk` can generate commits and PRs with **no API key and no per-call billing**, riding your existing subscription.

This is the **recommended way to use Codex** — it's lighter-weight than the `codex-cli` provider, which shells out to the full `codex` binary for every call.

```sh
# Sign in once with the Codex CLI (ChatGPT account), then:
sk commit --provider codex
sk pr --auto --provider codex
```

```yaml
# ~/.config/skald/config.yaml — make it your default
provider: codex
```

Notes and limits:

- **Requires a ChatGPT-mode Codex login.** If Codex is signed in with an API key instead, use the OpenAI-compatible API-key flow rather than `codex`. `sk doctor` reports which mode you're in.
- **Read-only credentials.** `sk` never writes back to `~/.codex/auth.json`. When the token expires, run `codex` to refresh it — `sk` will not rotate the shared token (doing so would break the Codex CLI).
- **`codex` vs `codex-cli`.** `codex` is this direct ChatGPT-subscription path; `codex-cli` is the CLI shell-out provider. They are independent — selecting one does not change the other.
- **Override the model** with `--model` or `providers.codex.model` (default `gpt-5.5`). The endpoint can be overridden with `--base-url`, `providers.codex.base_url`, or `CODEX_BASE_URL`.
- The backend is an internal Codex endpoint and may change without notice.

### API Key Setup

API keys can be set three ways (highest priority first):

1. **CLI flag:** `--api-key sk-ant-...`
2. **Config file:** `providers.anthropic.api_key`
3. **Environment variable:** `ANTHROPIC_API_KEY` (recommended)

```yaml
# ~/.config/skald/config.yaml
provider: anthropic

providers:
  anthropic:
    api_key: $ANTHROPIC_API_KEY
```

### Base URL Override

For custom endpoints (proxies, enterprise deployments):

```yaml
providers:
  anthropic:
    base_url: https://your-proxy.example.com
```

Or via `ANTHROPIC_BASE_URL` env var, or `--base-url` flag.

### Model Aliases

Short aliases resolve to the latest version of each model family:

| Alias | Resolves To |
|-------|-------------|
| `sonnet` | `claude-sonnet-4` |
| `opus` | `claude-opus-4` |
| `haiku` | `claude-haiku-4-5` |

Full model IDs (e.g., `claude-sonnet-4-20250514`) are also accepted for pinned versions.

### Diff Compaction

API providers automatically compact large diffs before sending:

1. **Smart filtering** — removes lock files, build output, generated code, and binary diffs
2. **File summarization** — if still over the token budget, summarizes the largest files

The original diff stat is always preserved so the model knows what changed. Compaction details are logged at `-vv` verbosity.

## CLI Providers

CLI providers shell out to an installed binary on your system.

| Provider | CLI Binary | Install |
|----------|-----------|---------|
| Claude | `claude` | [Claude Code](https://claude.ai/code) |
| `codex-cli` | `codex` | [Codex CLI](https://developers.openai.com/codex/cli) |
| Gemini | `gemini` | [Gemini CLI](https://github.com/google-gemini/gemini-cli) |
| OpenCode | `opencode` | [OpenCode](https://opencode.ai) |
| Copilot | `copilot` | [Copilot CLI](https://docs.github.com/copilot) |

> For Codex, prefer the [`codex`](#codex-via-your-chatgpt-subscription-codex) API provider above — it rides your ChatGPT subscription directly and is faster than `codex-cli`, which spawns the full `codex` binary on every call. Reach for `codex-cli` only if you specifically need the CLI's own behavior.

## Configuration

Set your default provider in config:

```yaml
# ~/.config/skald/config.yaml
provider: anthropic
```

For API providers, include credentials:

```yaml
provider: anthropic

providers:
  anthropic:
    api_key: $ANTHROPIC_API_KEY
    model: claude-sonnet-4
```

For CLI providers, set a specific model:

```yaml
providers:
  claude:
    model: claude-haiku-4-5
```

## CLI Overrides

Override the provider or model for a single command:

```sh
sk commit --provider anthropic
sk commit --provider anthropic --model sonnet
sk commit --provider codex               # Codex via ChatGPT subscription (no API key)
sk commit --provider gemini --model gemini-2.5-flash
sk pr --auto --provider anthropic --api-key sk-ant-...
sk pr --auto --provider anthropic --base-url https://your-proxy.example.com
```

## Setup

Run `sk config init` to set up your provider interactively, or specify directly:

```sh
sk config init --provider anthropic
sk config init --provider codex
```

The interactive picker lists **every** provider with its current readiness, so
the choice is always yours to make — nothing is hidden based on ambient state:

| Label | Meaning |
|-------|---------|
| `ready` | Usable now — CLI binary is on your PATH, or API credentials resolve. |
| `needs setup` | An API provider you can pick now; `sk` writes the config and tells you how to add credentials. |
| `not installed` | A CLI provider whose binary isn't on your PATH yet. |

Picking a `needs setup` or `not installed` provider still writes the config —
invoking the command is the intent — and prints the next step (set the API key,
or install the CLI). Readiness for API providers is computed from the same
credential chain `sk` uses at runtime (`--api-key` → `providers.<name>.api_key`
→ environment variable), so what you see is what will actually be used.

## Migrating from `codex-api` / `codex`

The Codex providers were renamed so the recommended path has the simplest name:

| Before | After | What it is |
|--------|-------|------------|
| `codex-api` | `codex` | Direct ChatGPT-subscription path (recommended) |
| `codex` | `codex-cli` | Shell-out to the `codex` binary |

To migrate:

- If you used **`codex-api`**, change it to **`codex`** (in config `provider:`/`providers:` keys, `--provider` flags, and aliases). `codex-api` is no longer accepted — `sk` will point you to `codex`.
- If you used **`codex`** (the CLI shell-out), change it to **`codex-cli`**. Note that a bare `codex` now means the ChatGPT-subscription path; if no ChatGPT login is found, `sk` errors and points you to `codex-cli`.

## Verification

```sh
sk doctor
```

Doctor checks all known providers and reports which are available.
