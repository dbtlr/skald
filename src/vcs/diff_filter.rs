pub const DEFAULT_EXCLUDES: &[&str] = &[
    "*.lock",
    "*-lock.json",
    "*.sum",
    "*.min.js",
    "*.min.css",
    "*.map",
    "*.generated.*",
    "dist/*",
    "build/*",
    "*.pyc",
    "*.o",
    "*.so",
];

/// Check if a path matches a simple glob pattern.
///
/// Supported patterns:
/// - `*.ext` — matches any file ending with `.ext`
/// - `dir/*` — matches any file under `dir/`
/// - `*-lock.json` — matches suffix after any prefix
/// - `*.generated.*` — matches files containing `.generated.` in the name
pub fn matches_glob(path: &str, pattern: &str) -> bool {
    if let Some(suffix) = pattern.strip_prefix('*') {
        // Pattern like "*.lock", "*-lock.json", "*.generated.*"
        // For "*.generated.*", we need to check the filename contains ".generated."
        if suffix.contains('*') {
            // e.g. "*.generated.*" -> suffix is ".generated.*"
            // strip the trailing * and check if the path contains the middle part
            if let Some(middle) = suffix.strip_suffix('*') {
                return path.contains(middle);
            }
        }
        path.ends_with(suffix)
    } else if let Some(prefix) = pattern.strip_suffix('*') {
        // Pattern like "dist/*"
        path.starts_with(prefix)
    } else {
        // Exact match
        path == pattern
    }
}

/// Filter a unified diff, removing hunks for files that match exclude patterns.
///
/// Parses the diff looking for `diff --git a/... b/path` headers.
/// If `b/path` matches any pattern, all lines until the next diff header are skipped.
pub fn filter_diff(diff: &str, exclude_patterns: &[String], include_defaults: bool) -> String {
    if diff.is_empty() {
        return String::new();
    }

    let mut result = String::with_capacity(diff.len());
    let mut skip = false;

    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            // Extract the b/path from "a/foo b/bar"
            let path = rest.rsplit_once(" b/").map(|(_, b)| b).unwrap_or("");

            skip = should_exclude(path, exclude_patterns, include_defaults);

            if !skip {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(line);
            }
        } else if !skip && (!result.is_empty() || !line.is_empty()) {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(line);
        }
    }

    result
}

fn should_exclude(path: &str, exclude_patterns: &[String], include_defaults: bool) -> bool {
    if include_defaults {
        for pattern in DEFAULT_EXCLUDES {
            if matches_glob(path, pattern) {
                return true;
            }
        }
    }

    for pattern in exclude_patterns {
        if matches_glob(path, pattern) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_files_filtered_out() {
        let diff = "\
diff --git a/Cargo.lock b/Cargo.lock
index abc..def 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -1,3 +1,4 @@
+new lock content
diff --git a/src/main.rs b/src/main.rs
index abc..def 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
+fn main() {}";

        let result = filter_diff(diff, &[], true);
        assert!(!result.contains("Cargo.lock"));
        assert!(result.contains("src/main.rs"));
        assert!(result.contains("fn main() {}"));
    }

    #[test]
    fn custom_patterns_work() {
        let diff = "\
diff --git a/vendor/lib.js b/vendor/lib.js
index abc..def 100644
--- a/vendor/lib.js
+++ b/vendor/lib.js
@@ -1,3 +1,4 @@
+vendored code
diff --git a/src/app.js b/src/app.js
index abc..def 100644
--- a/src/app.js
+++ b/src/app.js
@@ -1,3 +1,4 @@
+app code";

        let patterns = vec!["vendor/*".to_string()];
        let result = filter_diff(diff, &patterns, false);
        assert!(!result.contains("vendor/lib.js"));
        assert!(result.contains("src/app.js"));
    }

    #[test]
    fn defaults_can_be_disabled() {
        let diff = "\
diff --git a/Cargo.lock b/Cargo.lock
index abc..def 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -1,3 +1,4 @@
+lock content";

        let result = filter_diff(diff, &[], false);
        assert!(result.contains("Cargo.lock"));
    }

    #[test]
    fn extension_glob_matching() {
        assert!(matches_glob("foo.lock", "*.lock"));
        assert!(matches_glob("package-lock.json", "*-lock.json"));
        assert!(matches_glob("foo.min.js", "*.min.js"));
        assert!(matches_glob("types.generated.ts", "*.generated.*"));
        assert!(!matches_glob("foo.rs", "*.lock"));
    }

    #[test]
    fn directory_glob_matching() {
        assert!(matches_glob("dist/bundle.js", "dist/*"));
        assert!(matches_glob("build/output.css", "build/*"));
        assert!(!matches_glob("src/main.rs", "dist/*"));
    }

    #[test]
    fn empty_diff_returns_empty() {
        assert_eq!(filter_diff("", &[], true), "");
    }
}
