# skald-vcs

VCS adapter trait and implementations for [skald](https://github.com/dbtlr/skald).

> **Note:** This is an internal implementation crate for skald. It is not intended for direct external use and its API may change without notice.

## What it provides

- **VCS adapter trait** — interface for version control operations (diff, commit, staging, branching, pushing)
- **Git implementation** — full Git adapter via shell commands
- **Diff filtering** — lock file and binary exclusion for cleaner AI input

For more information, see the [skald repository](https://github.com/dbtlr/skald).

## License

MIT
