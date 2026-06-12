# Aliases

Aliases are composable flag shortcuts for skald commands. They are not shell aliases -- they expand within skald before argument parsing.

## Managing Aliases

### Adding an alias

```sh
sk alias add ci "commit -n 5"             # add to global config
sk alias add ci "commit -n 5" --project   # add to project config
sk alias add ci "commit -n 10" --force    # overwrite existing alias
```

### Removing an alias

```sh
sk alias remove ci             # remove from global config
sk alias remove ci --project   # remove from project config
```

### Listing aliases

```sh
sk alias list              # list all active aliases (always shows source)
sk alias list --format json
```

### Manual configuration

You can also define aliases directly in your config files:

```yaml
aliases:
  ci: "commit -n 5"
  ca: "commit -y -a"
  fix: "commit -y -a --context 'bug fix'"
```

Each alias maps a short name to a command with flags. When you run `sk ci`, skald expands it to `sk commit -n 5` before parsing.

## How Expansion Works

1. Skald loads config before parsing CLI arguments
2. If the first argument matches an alias name, it's replaced with the expansion
3. Any additional arguments you pass are appended after the expansion
4. The expanded arguments are then parsed by clap normally

```sh
sk ci --dry-run
# expands to: sk commit -n 5 --dry-run
```

## Resolution Rules

Aliases follow the same merge rules as other config:

- **Project wins.** A project alias with the same name as a global alias replaces it.
- **Last-wins within a file.** Standard YAML duplicate-key behavior.
- Global aliases not overridden by the project are preserved.

## Restrictions

- **No shadowing builtins.** An alias cannot have the same name as a built-in command (`commit`, `pr`, `config`, `aliases`, `doctor`, `completions`).
- **No recursion.** An alias expansion cannot reference another alias as its first token.
- **Must target a builtin.** The first token of an alias expansion must be a built-in command.

Violating any of these produces a clear error when adding the alias.

## Examples

### Quick commit with more candidates

```sh
sk alias add ci "commit -n 5"
```

### Auto-commit all files

```sh
sk alias add ca "commit -y -a"
```

### Context-aware commits

```sh
sk alias add fix "commit -y -a --context 'bug fix'"
sk alias add feat "commit -y -a --context 'new feature'"
```

### PR shortcut

```sh
sk alias add p "pr"
```
