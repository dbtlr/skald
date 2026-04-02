/// Diff compaction pipeline for API providers.
///
/// Two-stage process:
/// 1. Smart filtering — remove noise (lock files, build output, generated code, binaries)
/// 2. File summarization — collapse largest file diffs when over token budget

/// Result of compacting a diff.
#[derive(Debug, Clone)]
pub struct CompactedDiff {
    pub diff: String,
    pub stat: String,
    pub was_compacted: bool,
    pub dropped_files: Vec<String>,
}

/// Default token budget (chars / 4 heuristic). Conservative for Sonnet's 200K context.
const DEFAULT_TOKEN_BUDGET: usize = 80_000;

/// Patterns for file paths that should be filtered from diffs.
const FILTERED_DIRECTORIES: &[&str] = &[
    "dist/", "build/", "out/", ".next/", "target/", "__pycache__/", "node_modules/",
    "generated/", "auto-generated/", "codegen/",
];

const FILTERED_EXTENSIONS: &[&str] = &[
    ".min.js", ".min.css", ".map",
];

const FILTERED_FILENAMES: &[&str] = &[
    "Cargo.lock", "package-lock.json", "yarn.lock", "pnpm-lock.yaml",
    "composer.lock", "Gemfile.lock", "poetry.lock",
];

/// Check if a file path matches any filter pattern.
fn should_filter(path: &str) -> bool {
    let path_lower = path.to_lowercase();

    // Check directory patterns
    for dir in FILTERED_DIRECTORIES {
        if path_lower.contains(dir) {
            return true;
        }
    }

    // Check *-generated/ pattern
    for segment in path_lower.split('/') {
        if segment.ends_with("-generated") {
            return true;
        }
    }

    // Check .generated. in filename
    if let Some(filename) = path.rsplit('/').next() {
        if filename.contains(".generated.") {
            return true;
        }
    }

    // Check filtered extensions
    for ext in FILTERED_EXTENSIONS {
        if path_lower.ends_with(ext) {
            return true;
        }
    }

    // Check filtered filenames
    if let Some(filename) = path.rsplit('/').next() {
        for name in FILTERED_FILENAMES {
            if filename == *name {
                return true;
            }
        }
    }

    false
}

/// Parse a unified diff into per-file sections.
/// Returns a vec of (file_path, section_text) tuples.
fn parse_diff_sections(diff: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let mut current_path = String::new();
    let mut current_section = String::new();

    for line in diff.lines() {
        if line.starts_with("diff --git ") {
            // Save previous section
            if !current_path.is_empty() {
                sections.push((current_path.clone(), current_section.clone()));
            }
            // Extract path from "diff --git a/path b/path"
            current_path = line
                .strip_prefix("diff --git a/")
                .and_then(|rest| rest.split(" b/").next())
                .unwrap_or("")
                .to_string();
            current_section = format!("{line}\n");
        } else {
            current_section.push_str(line);
            current_section.push('\n');
        }
    }

    // Don't forget the last section
    if !current_path.is_empty() {
        sections.push((current_path, current_section));
    }

    sections
}

/// Estimate token count using chars/4 heuristic.
fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

/// Compact a diff for sending to an API provider.
///
/// Stage 1: Filter out noise files (lock files, build output, generated code).
/// Stage 2: If still over budget, summarize the largest remaining file diffs.
pub fn compact_diff(diff: &str, stat: &str) -> CompactedDiff {
    compact_diff_with_budget(diff, stat, DEFAULT_TOKEN_BUDGET)
}

/// Compact a diff with a specific token budget.
pub fn compact_diff_with_budget(diff: &str, stat: &str, token_budget: usize) -> CompactedDiff {
    let sections = parse_diff_sections(diff);
    let mut dropped_files = Vec::new();

    // Stage 1: Smart filtering
    let mut kept: Vec<(String, String)> = Vec::new();
    for (path, section) in &sections {
        if should_filter(path) {
            dropped_files.push(path.clone());
            tracing::debug!(file = %path, "compaction: filtered out");
        } else if section.contains("Binary files") && section.lines().count() <= 3 {
            dropped_files.push(path.clone());
            tracing::debug!(file = %path, "compaction: filtered binary");
        } else {
            kept.push((path.clone(), section.clone()));
        }
    }

    let filtered_diff: String = kept.iter().map(|(_, s)| s.as_str()).collect();
    let filtered_tokens = estimate_tokens(&filtered_diff);

    if filtered_tokens <= token_budget {
        return CompactedDiff {
            diff: filtered_diff,
            stat: stat.to_string(),
            was_compacted: !dropped_files.is_empty(),
            dropped_files,
        };
    }

    // Stage 2: Summarize largest files until under budget
    // Sort by size descending
    kept.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    let mut result_sections: Vec<(String, String)> = kept.clone();
    let mut current_tokens = filtered_tokens;

    for (path, section) in &kept {
        if current_tokens <= token_budget {
            break;
        }

        let section_tokens = estimate_tokens(section);

        // Build a summary: stat line + first few lines of context
        let summary_lines: Vec<&str> = section
            .lines()
            .take(10) // diff header + first hunk header + a few context lines
            .collect();
        let summary = format!(
            "{}\n... (large diff summarized, {} lines omitted)\n",
            summary_lines.join("\n"),
            section.lines().count().saturating_sub(10)
        );
        let summary_tokens = estimate_tokens(&summary);

        // Replace in result_sections
        if let Some(entry) = result_sections.iter_mut().find(|(p, _)| p == path) {
            entry.1 = summary;
            current_tokens = current_tokens - section_tokens + summary_tokens;
            dropped_files.push(path.clone());
            tracing::debug!(file = %path, saved_tokens = section_tokens - summary_tokens, "compaction: summarized");
        }
    }

    let compacted_diff: String = result_sections.iter().map(|(_, s)| s.as_str()).collect();

    CompactedDiff {
        diff: compacted_diff,
        stat: stat.to_string(),
        was_compacted: true,
        dropped_files,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_filter_lock_files() {
        assert!(should_filter("Cargo.lock"));
        assert!(should_filter("package-lock.json"));
        assert!(should_filter("yarn.lock"));
        assert!(should_filter("pnpm-lock.yaml"));
        assert!(should_filter("composer.lock"));
        assert!(should_filter("Gemfile.lock"));
        assert!(should_filter("poetry.lock"));
    }

    #[test]
    fn should_filter_build_directories() {
        assert!(should_filter("dist/bundle.js"));
        assert!(should_filter("build/output.css"));
        assert!(should_filter("out/main.js"));
        assert!(should_filter(".next/cache/data.json"));
        assert!(should_filter("target/debug/binary"));
        assert!(should_filter("__pycache__/module.pyc"));
        assert!(should_filter("node_modules/pkg/index.js"));
    }

    #[test]
    fn should_filter_generated_patterns() {
        assert!(should_filter("src/generated/types.ts"));
        assert!(should_filter("auto-generated/schema.rs"));
        assert!(should_filter("codegen/output.go"));
        assert!(should_filter("api-generated/client.py"));
        assert!(should_filter("src/types.generated.ts"));
    }

    #[test]
    fn should_filter_minified_and_maps() {
        assert!(should_filter("app.min.js"));
        assert!(should_filter("styles.min.css"));
        assert!(should_filter("bundle.js.map"));
    }

    #[test]
    fn should_not_filter_source_files() {
        assert!(!should_filter("src/main.rs"));
        assert!(!should_filter("lib/utils.ts"));
        assert!(!should_filter("README.md"));
        assert!(!should_filter("Cargo.toml"));
        assert!(!should_filter("src/generator.rs")); // "generator" != "generated"
    }

    #[test]
    fn parse_diff_sections_splits_correctly() {
        let diff = "\
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,3 @@
-old line
+new line
 context
diff --git a/Cargo.lock b/Cargo.lock
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -1,5 +1,5 @@
-old lock
+new lock
";
        let sections = parse_diff_sections(diff);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].0, "src/main.rs");
        assert_eq!(sections[1].0, "Cargo.lock");
    }

    #[test]
    fn compact_filters_lock_files() {
        let diff = "\
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,3 @@
-old line
+new line
 context
diff --git a/Cargo.lock b/Cargo.lock
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -1,5 +1,5 @@
-old lock
+new lock
";
        let stat = " src/main.rs | 1 +\n Cargo.lock  | 1 +\n";
        let result = compact_diff(diff, stat);

        assert!(result.was_compacted);
        assert!(result.dropped_files.contains(&"Cargo.lock".to_string()));
        assert!(result.diff.contains("src/main.rs"));
        assert!(!result.diff.contains("Cargo.lock"));
        // Stat is always preserved
        assert!(result.stat.contains("Cargo.lock"));
    }

    #[test]
    fn compact_no_change_when_nothing_to_filter() {
        let diff = "\
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,3 @@
-old line
+new line
";
        let stat = " src/main.rs | 1 +\n";
        let result = compact_diff(diff, stat);

        assert!(!result.was_compacted);
        assert!(result.dropped_files.is_empty());
    }

    #[test]
    fn compact_summarizes_large_files_over_budget() {
        // Create a diff that's over a small budget
        let mut large_diff = String::from("diff --git a/src/big.rs b/src/big.rs\n--- a/src/big.rs\n+++ b/src/big.rs\n@@ -1,100 +1,100 @@\n");
        for i in 0..200 {
            large_diff.push_str(&format!("+line {i}\n"));
        }

        let stat = " src/big.rs | 200 +\n";
        // Use a very small budget to trigger summarization
        let result = compact_diff_with_budget(&large_diff, stat, 100);

        assert!(result.was_compacted);
        assert!(result.diff.contains("large diff summarized"));
        assert!(result.diff.len() < large_diff.len());
    }

    #[test]
    fn estimate_tokens_heuristic() {
        // 400 chars = ~100 tokens
        let text = "a".repeat(400);
        assert_eq!(estimate_tokens(&text), 100);
    }
}
