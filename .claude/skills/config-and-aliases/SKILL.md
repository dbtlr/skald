---
name: config-and-aliases
description: Use when working on the config system, alias resolution, config file loading/merging, XDG paths, env var expansion, project config discovery, or the config/aliases commands.
---

# Config & Aliases Guidelines

## Config Values vs Aliases

This is a hard boundary — never blur it:

- **Config values** = context (provider, model, language, pr_target, platform, vcs). Facts about the environment.
- **Aliases** = behavior (flag compositions). Composable command shortcuts.

There are **no configurable defaults** for command behavior. Bare commands always behave identically everywhere.

## Specificity Chain

Resolution order for all configurable values (most specific wins):

```
CLI flag  →  project config (.skaldrc.yaml)  →  global config (~/.config/skald/config.yaml)  →  built-in default
```

Never deviate from this order. It applies to context values, prompt templates, and aliases.

## Alias Resolution Rules

1. Check if the command name matches a known alias.
2. If alias exists in both global and project config, **project wins entirely** (no merging).
3. Expand: replace alias with its definition string.
4. Append explicit CLI args after the expansion.
5. Last-wins: conflicting flags resolve left-to-right, last value wins.

Aliases cannot: reference other aliases (no recursion), shadow builtin commands, contain arbitrary shell commands.

## Config File Locations

- Global: `$XDG_CONFIG_HOME/skald/config.yaml` (default: `~/.config/skald/config.yaml`)
- Global prompts: `~/.config/skald/prompts/`
- Project: `.skaldrc.yaml` in repo root
- Project prompts: `.skald/prompts/` in repo root
- Windows: `%LOCALAPPDATA%\skald\`

## Env Var Expansion

API keys and sensitive values support `$ENV_VAR` syntax in YAML config. Resolved at runtime, never written to disk. If the env var is not set, produce a clear error naming the variable.

## Config Loading

- On startup, load global config first, then discover project config by walking up from cwd.
- Project context values override global context values (merge at the key level, not replace the whole file).
- Project aliases replace global aliases of the same name entirely.
- If no config exists at all, the tool should still work with built-in defaults. Don't error on missing config — suggest `sk config init`.

## Positive/Negative Flag Pairs

Boolean behavioral flags have both forms:

| Positive | Negative | Default |
|----------|----------|---------|
| `--interactive` | `--no-interactive` | `--interactive` |
| `--extended` | `--no-extended` | `--no-extended` |
| `--auto` | `--no-auto` | `--no-auto` |
| `--draft` | `--no-draft` | `--no-draft` |
| `--color` | `--no-color` | `--color` |

This is essential for alias override ergonomics.
