# Prompts

Skald uses a template system for all AI prompts. Every prompt is a [Tera](https://keats.github.io/tera/docs/) template that gets rendered with context variables before being sent to the AI provider.

## Built-in Templates

| Template | Purpose |
|---|---|
| `system` | System message prepended to all AI calls |
| `commit-title` | Generates conventional commit message one-liners |
| `commit-body` | Generates extended commit body descriptions |
| `pr-title` | Generates pull request title suggestions |
| `pr-description` | Generates structured PR body (What/Why/Key Changes/Testing) |

## Resolution Chain

When skald needs a prompt template, it searches in this order:

1. **CLI flag** — direct file path via `--prompt` flag (future)
2. **Project config** — `.skald/prompts/<name>.md` in the project root
3. **Global config** — `~/.config/skald/prompts/<name>.md`
4. **Built-in default** — compiled into the binary

The first match wins. This lets you override prompts per-project or globally without modifying the binary.

## Template Variables

| Variable | Available In | Description |
|---|---|---|
| `branch` | all | Current git branch name |
| `target_branch` | PR prompts | PR target branch (e.g., `main`) |
| `diff_stat` | all | Output of `git diff --stat` |
| `context` | all | User-provided `--context` string |
| `language` | all | Configured language (default: English) |
| `num_suggestions` | commit-title, pr-title | Number of suggestions to generate |
| `commit_log` | PR prompts | Commit log for the branch |
| `title` | commit-body | The selected commit title |
| `files_changed` | all | Comma-separated list of changed file paths |

## Ejecting Templates

To customize prompts, eject the built-in templates to disk:

```bash
# Eject all templates to global config
sk config eject

# Eject a single template
sk config eject commit-title

# Eject to project directory (.skald/prompts/)
sk config eject --project
sk config eject --project commit-title
```

Ejected files include a header comment documenting all available variables. Edit freely — skald will use your file instead of the built-in default. Delete the file to revert to defaults.

Existing files are never overwritten by eject.

## Debugging Prompts

To see the rendered prompt sent to the AI, use trace-level verbosity:

```bash
sk commit -vvv
sk pr -vvv
```

The fully rendered prompt (with real diff, context, and template variables resolved) is logged at trace level. Check `~/.config/skald/logs/` or stderr with `-vvv` to inspect it.

## Tera Syntax

Templates use [Tera](https://keats.github.io/tera/docs/) syntax. Key features:

- **Variables**: `{{ branch }}`, `{{ diff_stat }}`
- **Conditionals**: `{% if context %}...{% endif %}`
- **Comparisons**: `{% if language != "English" %}...{% endif %}`
- **Comments**: `{# This is a comment #}`
