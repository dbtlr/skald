# PR

`sk pr` generates AI-powered PR titles and descriptions from your branch diff and commit history.

## Usage

```sh
sk pr [options]
```

## Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--auto` | | Generate title + description and create the PR immediately |
| `--title-only` | | Print title suggestions to stdout without creating the PR |
| `--dry-run` | | Print the full PR payload (title + body) without creating the PR |
| `--draft` | | Create the PR as a draft |
| `--push` | | Push the current branch to remote before creating the PR |
| `--update` | | Update an existing PR (coming in M9) |
| `--base` | `-b` | Target branch to merge into (overrides config `pr_target`) |
| `--num` | `-n` | Number of title suggestions to generate (default: 3) |
| `--context` | `-c` | Provide extra context to guide the AI |
| `--show-prompt` | | Render the PR prompt template and print to stdout without calling AI |
| `--format` | | Output format: `plain`, `table`, `json` |

## Examples

### Auto-create a PR

Generate a title and description, then create the PR immediately:

```sh
sk pr --auto
```

### Draft PR with push

Push the branch and open a draft PR in one step:

```sh
sk pr --auto --draft --push
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

### Title suggestions only

Get multiple title candidates to choose from:

```sh
sk pr --title-only
sk pr --title-only -n 5
```

### Add context

Give the AI a hint about the intent of the PR:

```sh
sk pr --auto --context "refactored auth to use JWT tokens"
```

### Custom base branch

Target a branch other than the configured default:

```sh
sk pr --auto --base develop
```

### Inspect the prompt

See what would be sent to the AI without calling it:

```sh
sk pr --show-prompt
```

## Existing PR Detection

When `--auto` is used, skald checks whether a PR already exists for the current branch before creating one. If a PR is found, it prints a warning with the PR number and URL and exits without creating a duplicate.

## Unpushed Commits Note

After creating a PR, if the current branch has commits that have not been pushed to the remote, skald prints a reminder:

```
You have unpushed commits. Use `sk pr --push --update` to push and update the PR.
```

Use `--push` with `--auto` to push before creating the PR in a single command.

## Base Branch Resolution

The target branch is resolved in this order:

1. `--base` / `-b` flag
2. `pr_target` in project config (`.skaldrc.yaml`)
3. `pr_target` in global config (`~/.config/skald/config.yaml`)
4. Built-in default: `main`

## Platform Support

`sk pr --auto` requires a supported platform to create PRs. See [platforms.md](platforms.md) for setup instructions and supported platforms.
