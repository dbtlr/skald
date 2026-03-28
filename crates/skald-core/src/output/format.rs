use serde::Serialize;
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum OutputFormat {
    Plain,
    Table,
    Json,
}

impl OutputFormat {
    pub fn render_rows(&self, headers: &[&str], rows: &[Vec<String>], is_tty: bool) -> String {
        match self {
            Self::Plain => render_plain(rows),
            Self::Table => render_table(headers, rows),
            Self::Json => render_json(headers, rows, is_tty),
        }
    }

    pub fn render_value<T: Serialize>(&self, value: &T, is_tty: bool) -> String {
        match self {
            Self::Plain | Self::Table => serde_json::to_string(value)
                .unwrap_or_else(|_| String::from("<serialization error>")),
            Self::Json => if is_tty {
                serde_json::to_string_pretty(value)
            } else {
                serde_json::to_string(value)
            }
            .unwrap_or_else(|_| String::from("<serialization error>")),
        }
    }
}

fn render_plain(rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    for row in rows {
        let line = row.join("\t");
        let _ = writeln!(out, "{line}");
    }
    out
}

fn render_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let col_count = headers.len();
    let mut widths = vec![0usize; col_count];

    for (i, h) in headers.iter().enumerate() {
        widths[i] = h.len();
    }
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    let mut out = String::new();

    // Header
    for (i, h) in headers.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        let _ = write!(out, "{:<width$}", h, width = widths[i]);
    }
    out.push('\n');

    // Separator
    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        for _ in 0..*w {
            out.push('─');
        }
    }
    out.push('\n');

    // Rows
    for (idx, row) in rows.iter().enumerate() {
        let num = format!("{:>3}", idx + 1);
        out.push_str(&num);
        out.push_str("  ");
        for (i, cell) in row.iter().enumerate() {
            if i > 0 {
                out.push_str("  ");
            }
            if i < col_count {
                let _ = write!(out, "{:<width$}", cell, width = widths[i]);
            }
        }
        out.push('\n');
    }
    out
}

fn render_json(headers: &[&str], rows: &[Vec<String>], is_tty: bool) -> String {
    let objects: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            let mut map = serde_json::Map::new();
            for (i, h) in headers.iter().enumerate() {
                let val = row.get(i).cloned().unwrap_or_default();
                map.insert((*h).to_string(), serde_json::Value::String(val));
            }
            serde_json::Value::Object(map)
        })
        .collect();

    let value = serde_json::Value::Array(objects);
    if is_tty { serde_json::to_string_pretty(&value) } else { serde_json::to_string(&value) }
        .unwrap_or_else(|_| String::from("[]"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_data() -> (Vec<&'static str>, Vec<Vec<String>>) {
        let headers = vec!["Key", "Value", "Source"];
        let rows = vec![
            vec!["provider".into(), "claude-cli".into(), "global".into()],
            vec!["model".into(), "claude-sonnet-4".into(), "project".into()],
        ];
        (headers, rows)
    }

    #[test]
    fn plain_format_one_item_per_line() {
        let (_, rows) = sample_data();
        let out = OutputFormat::Plain.render_rows(&[], &rows, false);
        let lines: Vec<&str> = out.trim().lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("provider"));
        assert!(lines[1].contains("model"));
    }

    #[test]
    fn table_format_has_header_and_separator() {
        let (headers, rows) = sample_data();
        let out = OutputFormat::Table.render_rows(&headers, &rows, true);
        let lines: Vec<&str> = out.trim().lines().collect();
        assert!(lines.len() >= 4); // header + separator + 2 rows
        assert!(lines[0].contains("Key"));
        assert!(lines[1].contains('─'));
    }

    #[test]
    fn json_format_parses_as_array() {
        let (headers, rows) = sample_data();
        let out = OutputFormat::Json.render_rows(&headers, &rows, false);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 2);
        assert_eq!(parsed[0]["Key"], "provider");
    }

    #[test]
    fn json_pretty_in_tty() {
        let (headers, rows) = sample_data();
        let compact = OutputFormat::Json.render_rows(&headers, &rows, false);
        let pretty = OutputFormat::Json.render_rows(&headers, &rows, true);
        assert!(pretty.len() > compact.len());
    }
}
