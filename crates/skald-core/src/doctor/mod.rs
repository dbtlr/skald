pub mod checks;

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
    Fixed,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Environment,
    Configuration,
    Provider,
    Maintenance,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub category: Category,
    pub name: String,
    pub status: CheckStatus,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub was: Option<String>,
}

impl CheckResult {
    pub fn pass(name: &str, detail: &str) -> Self {
        Self {
            category: Category::Environment,
            name: name.to_string(),
            status: CheckStatus::Pass,
            detail: detail.to_string(),
            suggestion: None,
            was: None,
        }
    }

    pub fn warn(name: &str, detail: &str) -> Self {
        Self {
            category: Category::Environment,
            name: name.to_string(),
            status: CheckStatus::Warn,
            detail: detail.to_string(),
            suggestion: None,
            was: None,
        }
    }

    pub fn fail(name: &str, detail: &str) -> Self {
        Self {
            category: Category::Environment,
            name: name.to_string(),
            status: CheckStatus::Fail,
            detail: detail.to_string(),
            suggestion: None,
            was: None,
        }
    }

    pub fn fixed(name: &str, detail: &str) -> Self {
        Self {
            category: Category::Environment,
            name: name.to_string(),
            status: CheckStatus::Fixed,
            detail: detail.to_string(),
            suggestion: None,
            was: None,
        }
    }

    pub fn with_category(mut self, category: Category) -> Self {
        self.category = category;
        self
    }

    pub fn with_suggestion(mut self, suggestion: &str) -> Self {
        self.suggestion = Some(suggestion.to_string());
        self
    }

    pub fn with_was(mut self, was: &str) -> Self {
        self.was = Some(was.to_string());
        self
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub pass: usize,
    pub warn: usize,
    pub fail: usize,
    pub fixed: usize,
}

impl Summary {
    pub fn from_results(results: &[CheckResult]) -> Self {
        let mut pass = 0;
        let mut warn = 0;
        let mut fail = 0;
        let mut fixed = 0;
        for r in results {
            match r.status {
                CheckStatus::Pass => pass += 1,
                CheckStatus::Warn => warn += 1,
                CheckStatus::Fail => fail += 1,
                CheckStatus::Fixed => fixed += 1,
            }
        }
        Self { pass, warn, fail, fixed }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub checks: Vec<CheckResult>,
    pub summary: Summary,
}

pub fn run_checks(fix: bool, full: bool) -> DoctorReport {
    let mut results = Vec::new();

    results.extend(checks::environment_checks());
    results.extend(checks::config_checks(fix));
    results.extend(checks::provider_checks(full));
    results.extend(checks::maintenance_checks(fix));

    let summary = Summary::from_results(&results);
    DoctorReport { checks: results, summary }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_constructor() {
        let r = CheckResult::pass("git", "found");
        assert_eq!(r.status, CheckStatus::Pass);
        assert_eq!(r.name, "git");
        assert_eq!(r.detail, "found");
        assert!(r.suggestion.is_none());
        assert!(r.was.is_none());
    }

    #[test]
    fn warn_constructor() {
        let r = CheckResult::warn("gh", "not found");
        assert_eq!(r.status, CheckStatus::Warn);
    }

    #[test]
    fn fail_constructor() {
        let r = CheckResult::fail("git", "missing");
        assert_eq!(r.status, CheckStatus::Fail);
    }

    #[test]
    fn fixed_constructor() {
        let r = CheckResult::fixed("config_dir", "created");
        assert_eq!(r.status, CheckStatus::Fixed);
    }

    #[test]
    fn with_category_builder() {
        let r = CheckResult::pass("test", "ok").with_category(Category::Configuration);
        assert_eq!(r.category, Category::Configuration);
    }

    #[test]
    fn with_suggestion_builder() {
        let r = CheckResult::fail("test", "bad").with_suggestion("try this");
        assert_eq!(r.suggestion.unwrap(), "try this");
    }

    #[test]
    fn with_was_builder() {
        let r = CheckResult::fixed("test", "ok now").with_was("was broken");
        assert_eq!(r.was.unwrap(), "was broken");
    }

    #[test]
    fn summary_counts() {
        let results = vec![
            CheckResult::pass("a", "ok"),
            CheckResult::pass("b", "ok"),
            CheckResult::warn("c", "meh"),
            CheckResult::fail("d", "bad"),
            CheckResult::fixed("e", "fixed"),
            CheckResult::fixed("f", "fixed"),
        ];
        let s = Summary::from_results(&results);
        assert_eq!(s.pass, 2);
        assert_eq!(s.warn, 1);
        assert_eq!(s.fail, 1);
        assert_eq!(s.fixed, 2);
    }

    #[test]
    fn json_serialization() {
        let r = CheckResult::pass("git", "v2.40")
            .with_category(Category::Environment)
            .with_suggestion("upgrade");
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["status"], "pass");
        assert_eq!(json["category"], "environment");
        assert_eq!(json["suggestion"], "upgrade");
    }

    #[test]
    fn summary_json() {
        let s = Summary { pass: 3, warn: 1, fail: 0, fixed: 2 };
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["pass"], 3);
        assert_eq!(json["fixed"], 2);
    }

    #[test]
    fn report_json() {
        let results = vec![CheckResult::pass("git", "ok")];
        let report = DoctorReport { summary: Summary::from_results(&results), checks: results };
        let json = serde_json::to_value(&report).unwrap();
        assert!(json["checks"].is_array());
        assert_eq!(json["summary"]["pass"], 1);
    }
}
