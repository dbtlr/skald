use console::Style;
use std::sync::OnceLock;

static NO_COLOR: OnceLock<bool> = OnceLock::new();

pub fn init(no_color: bool) {
    let _ = NO_COLOR.set(no_color);
}

fn is_no_color() -> bool {
    *NO_COLOR.get().unwrap_or(&false)
}

pub fn success() -> Style {
    if is_no_color() { Style::new() } else { Style::new().green() }
}

pub fn warning() -> Style {
    if is_no_color() { Style::new() } else { Style::new().yellow() }
}

pub fn error() -> Style {
    if is_no_color() { Style::new() } else { Style::new().red() }
}

pub fn info() -> Style {
    if is_no_color() { Style::new() } else { Style::new().cyan() }
}

pub fn dim() -> Style {
    if is_no_color() { Style::new() } else { Style::new().dim() }
}
