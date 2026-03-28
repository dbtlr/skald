---
name: error-handling
description: Use when implementing error types, error formatting, user-facing error messages, recovery logic, or diagnostic output. Also use when adding new failure modes to existing commands or when working on the doctor command.
---

# Error Handling Guidelines

## Recovery Hierarchy

Every error path should follow this hierarchy, in order:

1. **Auto-fix if safe.** Missing config directory? Create it silently and continue. No staged changes but unstaged exist? Offer to stage them interactively.
2. **Suggest a fix if possible.** Every error that has a known remediation should include the fix in the message. Examples:
   - `claude CLI not found` → "Install with: npm install -g @anthropic-ai/claude-code"
   - `gh not authenticated` → "Run: gh auth login"
   - `Config parse error` → show the line number, the problematic value, and what was expected
3. **Fail clearly with context.** Say what was attempted, what went wrong, and where to look for more information. At `-v`, include the log file path.

## Error Types

Use `thiserror` for all error definitions in `skald-core`. Error types should be specific enough to pattern-match on:

```rust
// Good: specific, actionable
#[error("Provider '{provider}' not configured. Run `sk config init` to set up.")]
ProviderNotConfigured { provider: String },

// Bad: generic, useless to the caller
#[error("Configuration error: {0}")]
ConfigError(String),
```

## Formatting

- Errors print to stderr, never stdout (stdout is for data output).
- Use cliclack's `log::error()` and `log::warning()` for styled error output in TTY mode.
- In piped/non-TTY mode, errors are plain text to stderr.
- Never print a raw error code, stack trace, or Rust panic message at default verbosity.
- At `-v`, append: "See log file at ~/.config/skald/logs/skald-YYYY-MM-DD.log for details"
- At `-vv`, include the full error chain inline.

## Logging on Errors

When an error occurs, always log the full context at DEBUG level before formatting the user-facing message. The log file should contain:
- What operation was attempted
- The full error chain (including source errors)
- Relevant state (which config was loaded, what provider was selected, etc.)
- Timing (how long the operation ran before failing)

## Exit Codes

- `0` — success
- `1` — general error (config issues, provider errors, etc.)
- `2` — usage error (bad flags, missing arguments)
- `130` — interrupted (Ctrl+C / Esc in interactive mode)

## Never Panic

Use `Result` everywhere. The binary's `main()` should be the only place that unwraps, and it should catch and format the error using the error formatting infrastructure. A panic in a CLI tool is a bug.
