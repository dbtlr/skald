# Commit

`sk commit` generates AI-powered commit messages from your staged changes.

## Non-Interactive Modes

### Auto mode

Generate a single commit message and commit immediately:

```sh
sk commit --auto
```

### Message-only mode

Print generated messages to stdout without committing:

```sh
sk commit --message-only
```

Control the number of suggestions with `-n`:

```sh
sk commit --message-only -n 5
```

## Staging

Stage tracked modified files before committing:

```sh
sk commit --auto -a
```

Stage all files including untracked:

```sh
sk commit --auto -A
```

## Amend

Amend the previous commit with a newly generated message:

```sh
sk commit --auto --amend
```

## Context

Provide extra context to guide the AI:

```sh
sk commit --auto --context "Refactored auth to use JWT"
```

Or read context from a file:

```sh
sk commit --auto --context-file notes.txt
```

## Dry Run

See what would be committed without actually committing:

```sh
sk commit --auto --dry-run
```

## Show Prompt

Render the prompt template that would be sent to the AI, without calling it:

```sh
sk commit --show-prompt
```

## Output Formats

When using `--message-only`, control the output format:

```sh
sk commit --message-only --format plain   # one message per line (default)
sk commit --message-only --format table   # tabular output
sk commit --message-only --format json    # JSON array
```

## Interactive Mode (Default)

When you run `sk commit` without `--auto` or `--message-only`, skald enters an interactive carousel:

1. Analyzes staged changes
2. Generates commit message suggestions via AI
3. Presents a carousel for selection

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `←` `→` | Cycle through suggestions |
| `a` / Enter | Accept and commit |
| `e` | Edit the message inline |
| `?` | Show action menu (accept, amend, abort) |
| Esc / Ctrl+C | Abort |

### No Staged Changes

If no changes are staged, skald detects unstaged files and offers to stage them:
- **Stage all (-A)** — includes new and modified files
- **Stage tracked (-a)** — modified files only
- **Abort** — exit without staging

This only appears in interactive (TTY) mode. In non-interactive mode, use `-a` or `-A` flags.
