---
name: dependencies
description: Use when adding, evaluating, or updating Rust crate dependencies. Also use when considering whether to implement something yourself vs pulling in a library.
---

# Dependency Guidelines

## When to Add a Dependency

Add a crate when:
- It solves a non-trivial problem that would take > 100 lines to implement correctly (e.g., YAML parsing, terminal rendering, CLI arg parsing).
- It's well-maintained: published within the last 6 months, has multiple maintainers or is from a trusted org.
- It has meaningful adoption (check download counts on crates.io).

Implement it yourself when:
- The equivalent code is < ~50 lines.
- The crate would pull in a large dependency tree for a small feature.
- Examples: simple config path resolution, basic string formatting, small utility functions.

## Evaluation Criteria

Before running `cargo add`, check:
1. **Last publish date** — stale crates (> 1 year) are a risk.
2. **Download count** — very low downloads may indicate abandoned or untested code.
3. **Bus factor** — single maintainer crates are riskier. Check if the org has multiple contributors.
4. **Dependency tree** — run `cargo tree -p <crate>` to see what it pulls in. Avoid crates that bring in heavy transitive deps.
5. **Compile time impact** — proc macros and large crates (like syn-heavy derives) add noticeable build time. Weigh the value.
6. **Feature flags** — use minimal feature sets. e.g., `clap` with `features = ["derive"]` not the kitchen sink.

## Pinning Strategy

- Pin major versions in `Cargo.toml` (e.g., `serde = "1"`).
- Use `cargo update` deliberately, not automatically. Review changelogs.
- Run `cargo audit` periodically to check for known vulnerabilities.

## Key Dependencies (Established)

These are the chosen crates for skald — don't replace them without discussion:

| Purpose | Crate | Notes |
|---------|-------|-------|
| CLI framework | `clap` (derive) | Arg parsing, help generation |
| Shell completions | `clap_complete` | Generated from clap definitions |
| Interactive TUI | `cliclack` | Prompts, spinners, theming |
| Terminal styling | `console` | Used by cliclack internally |
| Serialization | `serde` + `serde_yaml` + `serde_json` | Config and output |
| Logging facade | `tracing` | Structured logging |
| Log file output | `tracing-subscriber` + `tracing-appender` | File rotation, filtering |
| Error handling | `thiserror` | Derive macros for error types |
| Config dirs | `dirs` | XDG/platform-aware paths |
| Async runtime | `tokio` | For future provider calls |
| Async traits | `async-trait` | Provider trait definitions |

## Compile Time Awareness

- Profile build times periodically with `cargo build --timings`.
- Clean builds for skald should stay under 60 seconds.
- If a new dependency adds > 10 seconds to clean build time, evaluate alternatives.
- Prefer crates that offer feature flags to slim their footprint.
