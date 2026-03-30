# Alias Add/Remove Operations

**Date:** 2026-03-30
**Status:** Approved

## Summary

Add `add` and `remove` subcommands to `sk alias` so users can manage aliases from the CLI without manually editing YAML config files. The existing bare `sk alias` (list) behavior moves behind an explicit `list` subcommand; bare `sk alias` prints help.

## CLI Interface

```
sk alias                                    # prints help
sk alias list [--source] [--format json]    # list all aliases
sk alias add <name> <expansion> [--project] [--force]
sk alias remove <name> [--project]
```

### Flags

- `--project` — Target `.skaldrc.yaml` instead of global config. Default is global (`~/.config/skald/config.yaml`).
- `--force` — Allow overwriting an existing alias. Without this, `add` fails if the alias already exists.
- `--source` — Show which config file defines each alias (on `list` only).
- `--format` — Output format: `plain`, `table`, `json` (on `list` only).

### Positional Arguments

- `add <name> <expansion>` — Name is the alias key, expansion is the full command string (e.g. `"commit -n 5"`).
- `remove <name>` — Name of the alias to remove.

## Config File Operations

### Target File Resolution

- **Default:** Global config at `global_config_path().join("config.yaml")`.
- **With `--project`:** Project config at `.skaldrc.yaml`, discovered via `discover_project_config()`.

### Add Behavior

1. Read the target YAML file (or start with empty `RawConfig` if file doesn't exist).
2. Check if alias already exists in the map. If so, fail unless `--force`.
3. Insert the alias into the `aliases` map.
4. Run `validate_aliases()` on the full merged alias set — catches shadowing, recursion, and invalid commands before writing.
5. Write the updated config back to the file.

### Remove Behavior

1. Read the target YAML file. If the file doesn't exist, error.
2. Check if the alias exists in the map. If not, error.
3. Remove the alias from the map.
4. Write the updated config back to the file.

### File Creation

- `add` with no existing config file: creates the file with just the `aliases` section.
- `add --project` with no existing project config: creates `.skaldrc.yaml` in the current directory.
- `remove` with no existing config file: errors with path context.

### YAML Roundtrip

Use `serde_yaml_ng` to deserialize into `RawConfig`, mutate, and serialize back. Comments are not preserved — this is consistent and predictable.

## Error Cases

| Scenario | Message |
|----------|---------|
| `add` when alias exists (no `--force`) | `Alias 'ci' already exists (expands to "commit -n 5"). Use --force to overwrite.` |
| `remove` when alias not found | `Alias 'ci' not found in global config.` / `...in project config.` |
| Validation: shadows builtin | Existing `AliasShadowsBuiltin` error |
| Validation: recursive | Existing `AliasRecursive` error |
| Validation: invalid command | Existing `AliasInvalidCommand` error |
| Config file not writable | `Cannot write to <path>: <os error>. Check file permissions.` |
| `remove` with no config file | `No config file found at <path>.` |

## Architecture

### Crate Changes

- **`skald-core`** — New `config/writer.rs` module:
  - `pub fn add_alias(name: &str, expansion: &str, path: &Path, force: bool) -> Result<()>`
  - `pub fn remove_alias(name: &str, path: &Path) -> Result<()>`
  - Handles the read-mutate-validate-write cycle.

- **`skald` (CLI)** — Update existing files:
  - `cli/root.rs` — Restructure `Alias` variant from flat struct to enum with `List`, `Add`, `Remove` subcommands.
  - `cli/aliases.rs` — Handle new subcommands, delegate to `skald-core` for write operations.

- **No changes** to `skald-providers` or `skald-vcs`.

### Scope

- Strictly non-interactive — positional args only, no prompts.
- One alias at a time for both `add` and `remove`.
- Operates on the specified scope only (global or project) — no cross-scope awareness.

## Design Decisions

1. **Bare `sk alias` prints help** — Consistent with `sk config`. Requires explicit `list` subcommand.
2. **Fail-by-default on overwrite** — Prevents accidental clobbering. `--force` is the escape hatch.
3. **Validate before write** — Catches errors at authoring time, not next invocation.
4. **No comment preservation** — Simpler implementation, consistent behavior. Users who care about comments can edit YAML directly.
5. **Global scope by default** — Aliases are typically personal preferences, not project-specific.
