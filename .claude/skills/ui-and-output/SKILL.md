---
name: ui-and-output
description: Use when working on terminal UI, cliclack prompts, output formatting, color handling, spinners, interactive flows, accessibility, or anything that renders to the terminal. Also use when adding new commands that produce user-visible output.
---

# UI & Output Guidelines

## cliclack Usage

Skald uses cliclack for all interactive terminal output. Every user-facing flow should use cliclack primitives consistently:

- `intro()` / `outro()` for session framing
- `spinner()` for any operation taking > 200ms (AI calls always, git on large repos conditionally)
- `select()` for choices with more than 2 options
- `confirm()` for yes/no decisions
- `input()` for free-text (context injection, editing)
- `log::info()`, `log::warning()`, `log::error()` for non-interactive status messages

## Custom Theme

A global cliclack `Theme` is set at startup. All prompts inherit it. The theme defines the visual identity — color palette, symbol style, spacing. Keep it consistent; never use raw ANSI codes outside the theme system.

## Output Format Contract

All commands that produce structured output support three formats:

- **`plain`** — one item per line, no decoration. Default when stdout is piped.
- **`table`** — numbered, aligned columns. Default in TTY.
- **`json`** — structured data via serde_json. Pretty-printed in TTY, compact when piped.

Use the shared formatters in `skald-core::output`. Never format output manually in command handlers.

## Color & Accessibility

- All color is routed through helpers that check the no-color state before applying styles.
- `NO_COLOR` env var (any value) → suppress all color unconditionally.
- `--color never` flag → same effect. Also supports `--color always` to force color through pipes.
- Piped stdout → auto no-color + no spinners + no interactive prompts.
- `$TERM=dumb` or unset → basic prompts only, no ANSI sequences.
- Never rely solely on color or symbols to convey meaning. Status text should be readable as plain text — "Error: config not found" not just a red ✗.

## Visual Hierarchy

- **◆ (diamond)** = decision point / action required
- **◇ (open diamond)** = completed step / status
- **▲ (triangle)** = warning
- **✗** = error
- **⚡** = auto-fixed
- The AI-generated message should always be the most visually prominent element in a flow.

## Interactive Commit Carousel

The commit message selection is a custom prompt (not a stock cliclack `Select`). It uses left/right arrow keys to cycle suggestions in-place, with single-key shortcuts on a bottom bar:
- `←` `→` cycle, `a` accept, `e` edit, `x` extend, `?` full action menu

Build this on crossterm for key input + cliclack's Theme for visual rendering.

## Performance

- Never add artificial delays or unnecessary output.
- Spinners should start immediately, not after a visible pause.
- If an operation is instant, show the result directly — don't spin for show.
