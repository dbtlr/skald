---
name: testing
description: Use when writing tests, deciding what to test, setting up test infrastructure, or reviewing test coverage. Also use when adding new commands or features that need test coverage.
---

# Testing Guidelines

## What to Test

- **Unit tests** for all core logic in `skald-core`: config loading/merging, alias resolution/expansion, output formatting, flag conflict resolution, env var expansion, path resolution.
- **Smoke tests** for the binary: assert commands run, exit with expected codes, produce expected output shapes. Use `assert_cmd` and `predicates` crates.
- **Integration tests** when the tool interacts with external systems (git, Claude CLI, gh). Add these as features mature, not preemptively. They may need fixtures, temp repos, or mocks.

## What Not to Test

- Trivial glue code with no branching logic.
- cliclack rendering (it's a third-party library — trust it).
- Exact output strings (fragile). Test output *shape* and *content*, not exact formatting.

## Test Organization

- Unit tests live in the same file as the code they test (Rust convention: `#[cfg(test)] mod tests`).
- Integration/smoke tests live in `crates/skald/tests/` (binary-level tests).
- Test helpers and fixtures go in a shared test utilities module if reused across crates.

## Testing Config

Config is the most test-worthy part of the core:
- Valid YAML loads correctly
- Missing config file → defaults work
- Malformed YAML → clear error with line info
- Env var expansion (`$API_KEY`) resolves when set, errors clearly when not
- Project config overrides global for context values
- Project alias replaces global alias of same name
- Unknown alias → helpful error (suggest closest match)

## Testing Aliases

Alias resolution has enough edge cases to warrant thorough coverage:
- Simple alias expands correctly
- Explicit CLI flags append after expansion
- Last-wins: `--extended --no-extended` → `--no-extended`
- Recursive alias detection → error
- Builtin shadowing detection → error
- Project alias overrides global alias of same name

## Testing Output Formats

The output format contract is a stability guarantee. Test that the same structured data renders correctly in all three formats (plain, table, json). The JSON schema should be asserted structurally.

## Commit Convention

Use conventional commit scopes that map to crate names: `test(core):`, `test(cli):`, etc.
