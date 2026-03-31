# skald-platform

Platform adapter trait and implementations for [skald](https://github.com/dbtlr/skald).

> **Note:** This is an internal implementation crate for skald. It is not intended for direct external use and its API may change without notice.

## What it provides

- **Platform adapter trait** — interface for code hosting platform operations (PR creation, updates, queries)
- **GitHub adapter** — full implementation via `gh` CLI
- **GitLab adapter** — full implementation via `glab` CLI
- **Platform detection** — automatic detection from git remote URLs

For more information, see the [skald repository](https://github.com/dbtlr/skald).

## License

MIT
