# CLAUDE.md

Skald (`sk`) is an AI-powered git workflow CLI written in Rust. It generates commit messages, PR titles, and PR descriptions.

## How We Work

- **Never push to main.** All work should be done in a brnach and pushed as a PR. No Exceptions.
- **Discuss first, code second.** It is harder to undo a mistake than it is to discuss options first.

## Core Principles

- **UX-first.** Every interaction should feel polished — including errors. Errors are suggestions, not complaints.
- **Specificity wins.** CLI flag → project config → global config → built-in default. Always. For prompts, aliases, and all configurable values.
- **Composability over configuration.** Bare commands have fixed defaults. Behavior is composed via aliases and flags. No hidden configurable defaults.
- **Fix it, don't complain.** Auto-fix if safe, suggest a fix if possible, fail clearly with context. Never a raw error without guidance.
- **Beautiful by default, quiet when piped.** cliclack for interactive, plain text when piped. Respect `NO_COLOR` unconditionally.
- **Observable.** Every shell-out, network call, and decision should be traceable at `-vv`/`-vvv`. Log files tell the full story.
- **Documentation is a deliverable.** Every PR that adds user-facing behavior must update `docs/`. If it's not documented, it's not done.
- **Performance matters.** Cold start under 50ms for non-AI commands. Lazy-initialize, avoid unnecessary work at startup.
- **Accessible.** NO_COLOR, dumb terminal fallback, screen-reader-friendly status messages, keyboard-only navigation.
- **Dogfood the tool.** Once M1 ships, use `sk` to generate commit messages and PR descriptions for skald itself.

## Project Structure

Single crate (`skald-cli`) with modules: `cli/` (commands), `ui/` (terminal UI), `engine/` (config, logging, output, prompts, doctor, errors), `providers/` (AI provider trait + impls), `vcs/` (VCS adapter), `platform/` (GitHub/GitLab adapters).

## Skills

Detailed development guidance lives in `.claude/skills/` and loads contextually. See those skills for guidance on UI/output, error handling, config/aliases, testing, and dependencies.
