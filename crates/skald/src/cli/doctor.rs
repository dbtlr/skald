use skald_core::doctor::{Category, CheckResult, CheckStatus, DoctorReport, run_checks};
use skald_core::output::OutputFormat;

fn category_name(cat: &Category) -> &'static str {
    match cat {
        Category::Environment => "Environment",
        Category::Configuration => "Configuration",
        Category::Provider => "Provider",
        Category::Maintenance => "Maintenance",
    }
}

fn status_symbol(status: &CheckStatus) -> &'static str {
    match status {
        CheckStatus::Pass => "✓",
        CheckStatus::Warn => "▲",
        CheckStatus::Fail => "✗",
        CheckStatus::Fixed => "⚡",
    }
}

fn render_check(check: &CheckResult) {
    let symbol = status_symbol(&check.status);
    cliclack::log::remark(format!("{symbol} {}: {}", check.name, check.detail)).ok();

    if let Some(ref suggestion) = check.suggestion {
        cliclack::log::remark(format!("  → {suggestion}")).ok();
    }
}

fn render_interactive(report: &DoctorReport) {
    let categories =
        [Category::Environment, Category::Configuration, Category::Provider, Category::Maintenance];

    for cat in &categories {
        let checks: Vec<&CheckResult> =
            report.checks.iter().filter(|c| &c.category == cat).collect();
        if checks.is_empty() {
            continue;
        }

        cliclack::log::remark(format!("\n{}", category_name(cat))).ok();

        for check in checks {
            render_check(check);
        }
    }

    // Summary
    let s = &report.summary;
    if s.fail == 0 && s.warn == 0 && s.fixed == 0 {
        cliclack::log::success("All checks passed.").ok();
    } else {
        let mut parts = Vec::new();
        if s.fixed > 0 {
            parts.push(format!("{} fixed", s.fixed));
        }
        if s.warn > 0 {
            parts.push(format!("{} warning{}", s.warn, if s.warn == 1 { "" } else { "s" }));
        }
        if s.fail > 0 {
            parts.push(format!("{} failure{}", s.fail, if s.fail == 1 { "" } else { "s" }));
        }
        let summary = parts.join(", ");
        if s.fail > 0 {
            cliclack::log::warning(summary).ok();
        } else {
            cliclack::log::info(summary).ok();
        }
    }
}

fn render_json(report: &DoctorReport, is_tty: bool) {
    let output = if is_tty {
        serde_json::to_string_pretty(report).expect("failed to serialize report")
    } else {
        serde_json::to_string(report).expect("failed to serialize report")
    };
    println!("{output}");
}

pub fn run_doctor(fix: bool, format: OutputFormat, is_tty: bool) -> i32 {
    let report = run_checks(fix);

    match format {
        OutputFormat::Json => render_json(&report, is_tty),
        _ => render_interactive(&report),
    }

    if report.summary.fail > 0 { 1 } else { 0 }
}
