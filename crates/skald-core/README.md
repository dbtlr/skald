# skald-core

Core library for [skald](https://github.com/dbtlr/skald) — config, logging, output, error types.

> **Note:** This is an internal implementation crate for skald. It is not intended for direct external use and its API may change without notice.

## What it provides

- **Config** — layered YAML config loading, merging, alias resolution, and config file writing
- **Prompts** — Tera-based prompt template system with built-in templates and eject support
- **Output** — format rendering (plain, table, JSON) with TTY detection
- **Logging** — tracing-based structured logging with file appender
- **Doctor** — environment and config validation checks
- **Error types** — unified error enum with user-facing suggestions

For more information, see the [skald repository](https://github.com/dbtlr/skald).

## License

MIT
