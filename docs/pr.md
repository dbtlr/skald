# PR

`sk pr` generates AI-powered PR titles and descriptions from your branch diff and commit history.

> **GitLab users:** Use `sk mr` instead of `sk pr` — both commands are identical, but `mr` reads more naturally for merge requests. Output automatically uses "MR" terminology when connected to GitLab.

## Usage

```sh
sk pr [options]
```

## Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--yes` | `-y` | Generate title + description and create/update immediately (implies `-n 1`) |
| `--dry-run` | | Print the full PR payload (title + body) without creating the PR |
| `--draft` | `-d` | Create the PR as a draft |
| `--push` | | Push the current branch to remote before creating the PR |
| `--base` | `-b` | Target branch to merge into (overrides config `pr_target`) |
| `--num` | `-n` | Number of title suggestions to generate (default: 3) |
| `--context` | `-c` | Provide extra context to guide the AI |
| `--context-file` | | Read context from a file |
| `--format` | | Output format: `plain`, `table`, `json` |

## Examples

### Auto-create a PR

Generate a title and description, then create the PR immediately:

```sh
sk pr -y
```

### Draft PR with push

Push the branch and open a draft PR in one step:

```sh
sk pr -y -d --push
```

### Preview with dry-run

See the generated title and body without creating anything:

```sh
sk pr --dry-run
```

### JSON output

Emit the PR payload as JSON (useful for scripting):

```sh
sk pr --dry-run --format json
```

### Add context

Give the AI a hint about the intent of the PR:

```sh
sk pr -y -c "refactored auth to use JWT tokens"
```

### Custom base branch

Target a branch other than the configured default:

```sh
sk pr -y -b develop
```

## Existing PR Detection

Skald automatically detects whether a PR already exists for the current branch:

- **If a PR exists**: updates its title and description with freshly generated content
- **If no PR exists**: creates a new one

This works in both `-y` mode and interactive mode. There is no separate `--update` flag — the tool does the right thing based on state.

## Unpushed Commits Note

After creating a PR, if the current branch has commits that have not been pushed to the remote, skald prints a reminder:

```
You have unpushed commits. Use `sk pr --push` to push and create/update the PR.
```

Use `--push` to push before creating the PR in a single command.

## Base Branch Resolution

The target branch is resolved in this order:

1. `--base` / `-b` flag
2. Existing PR's base branch (when updating)
3. `pr_target` in project config (`.skaldrc.yaml`)
4. `pr_target` in global config (`~/.config/skald/config.yaml`)
5. Built-in default: `main`

## Platform Support

`sk pr -y` requires a supported platform to create PRs. See [platforms.md](platforms.md) for setup instructions and supported platforms.

## Interactive Mode

Running `sk pr` without `-y` or `--dry-run` enters interactive mode.

### Stage 1: Title Selection

A carousel displays AI-generated title suggestions. Navigate with arrow keys:

| Key | Action |
|-----|--------|
| ← → | Cycle through suggestions |
| `a` / Enter | Accept title |
| `e` | Edit title inline |
| `?` | More options |
| Esc | Abort |

### Stage 2: Review and Confirm

After selecting a title, the full PR (title + body) is displayed for review:

| Option | Description |
|--------|-------------|
| Create / Update | Create or update the PR (based on existing PR detection) |
| Draft | Create as draft |
| Edit title | Edit the title inline |
| Edit body | Open body in `$EDITOR` |
| Context | Add context and regenerate |
| Abort | Exit without creating |

### Editor Setup

Body editing opens your preferred editor. Set `$VISUAL` or `$EDITOR`:

```sh
export VISUAL="code --wait"  # VS Code
export EDITOR="nvim"         # Neovim
```

Falls back to `vi` if neither is set.

### Diff Scope

Without `--push`, the generated content reflects only what's already pushed to the remote. With `--push`, local unpushed commits are included in the diff sent to the AI.
