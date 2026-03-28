# Aliases

Aliases are composable flag shortcuts for skald commands. They are not shell aliases -- they expand within skald before argument parsing.

## Defining Aliases

Add aliases to your global or project config file:

```yaml
aliases:
  ci: "commit -n 5"
  ca: "commit --auto -A"
  fix: "commit --auto -a --context 'bug fix'"
```

Each alias maps a short name to a command with flags. When you run `sk ci`, skald expands it to `sk commit -n 5` before parsing.

## How Expansion Works

1. Skald loads config before parsing CLI arguments
2. If the first argument matches an alias name, it's replaced with the expansion
3. Any additional arguments you pass are appended after the expansion
4. The expanded arguments are then parsed by clap normally

```sh
sk ci --no-extended
# expands to: sk commit -n 5 --no-extended
```

## Resolution Rules

Aliases follow the same merge rules as other config:

- **Project wins.** A project alias with the same name as a global alias replaces it.
- **Last-wins within a file.** Standard YAML duplicate-key behavior.
- Global aliases not overridden by the project are preserved.

## Viewing Aliases

```sh
sk aliases              # list all active aliases
sk aliases --source     # include which config file each alias comes from
sk aliases --format json
```

## Restrictions

- **No shadowing builtins.** An alias cannot have the same name as a built-in command (`commit`, `pr`, `config`, `aliases`, `doctor`, `completions`).
- **No recursion.** An alias expansion cannot reference another alias as its first token.
- **Must target a builtin.** The first token of an alias expansion must be a built-in command.

Violating any of these produces a clear error at config load time.

## Examples

### Quick commit with fewer candidates

```yaml
aliases:
  ci: "commit -n 5"
```

### Auto-commit (no interactive selection)

```yaml
aliases:
  ca: "commit --auto -A"
```

### Context-aware commit

```yaml
aliases:
  fix: "commit --auto -a --context 'bug fix'"
  feat: "commit --auto -a --context 'new feature'"
```

### PR shortcut

```yaml
aliases:
  p: "pr"
```
