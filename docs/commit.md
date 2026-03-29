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

## Interactive Mode

Interactive mode (`sk commit` with no flags) is planned for M5. For now, use `--auto` or `--message-only`.
