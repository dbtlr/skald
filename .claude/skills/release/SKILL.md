---
name: release
description: Skald release workflow. Use when user asks to "do a release", "release a new version", "cut a release", or wants to publish a new version to crates.io and GitHub.
metadata:
  internal: true
---

# Release Workflow

## Steps

1. **Run tests**: `cargo test && cargo clippy -- -D warnings && cargo fmt --check`
2. **Check current version**: Read `version` in `Cargo.toml`
3. **Review commits**: Check commits since last release to understand scope of changes
4. **Confirm release type with user**: Present changes summary and ask user to confirm patch/minor/major
5. **Bump version** (must run on a clean tree — before editing CHANGELOG):
   ```bash
   cargo release X.Y.Z --no-publish --no-push --no-tag --no-verify --no-confirm --execute && cargo check
   ```
   This bumps `Cargo.toml`, `Cargo.lock`, and auto-commits.
6. **Update CHANGELOG**: Add `## X.Y.Z` section at top with changes
7. **Commit**: Reset the auto-commit from step 5, stage everything, and create the final release commit:
   ```bash
   git reset --soft HEAD~1 && git add -A && git commit -m "Release vX.Y.Z"
   ```
8. **Push to main**: `git push origin main`
9. **Tag and push**: `git tag vX.Y.Z && git push origin vX.Y.Z`
10. **Wait for release workflow**: Poll with `gh run list --workflow=release.yml --limit 1` every 60 seconds until complete

The tag push triggers the release workflow which builds binaries, creates a GitHub release, and publishes to crates.io automatically.

## Confirm Release Type

**Before proceeding with changelog and version bump, confirm the release type with the user.**

After reviewing commits, present:
1. Current version (e.g., `0.3.1`)
2. Brief summary of changes (new features, bug fixes, breaking changes)
3. Your recommendation for release type with reasoning
4. The three options: patch, minor, major

Use `AskUserQuestion` to get explicit confirmation.

**Do not proceed until user confirms the release type.**

## CHANGELOG Format

```markdown
## X.Y.Z

### Added
- New feature description

### Changed
- Changed behavior description

### Fixed
- Bug fix description
```

**Section order:** Added, Changed, Fixed. Skip empty sections.

**Notable changes to document:**
- New features or commands
- User-visible behavior changes
- Bug fixes users might encounter

Skip: internal refactors, test additions, CI changes (unless they affect users).

## Version Guidelines

- **Patch** (0.3.1 → 0.3.2): Bug fixes, documentation, internal improvements
- **Minor** (0.3.1 → 0.4.0): New features, non-breaking behavior changes
- **Major** (0.3.1 → 1.0.0): Breaking changes (reserved for stable release)

## Troubleshooting

### Release workflow fails after tag push

If the workflow fails (e.g., cargo publish error), fix the issue, then recreate the tag:

```bash
gh release delete vX.Y.Z --yes           # Delete GitHub release
git push origin :refs/tags/vX.Y.Z        # Delete remote tag
git tag -d vX.Y.Z                        # Delete local tag
git tag vX.Y.Z && git push origin vX.Y.Z # Recreate and push
```

### cargo-release not installed

```bash
cargo install cargo-release
```
