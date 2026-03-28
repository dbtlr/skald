use cliclack::ThemeState;
use console::Style;

pub struct SkaldTheme;

impl SkaldTheme {
    pub fn apply() {
        cliclack::set_theme(Self);
    }
}

impl cliclack::Theme for SkaldTheme {
    fn bar_color(&self, state: &ThemeState) -> Style {
        match state {
            ThemeState::Active => Style::new().cyan(),
            ThemeState::Error(_) => Style::new().red(),
            ThemeState::Cancel => Style::new().red(),
            ThemeState::Submit => Style::new().green(),
        }
    }

    fn state_symbol_color(&self, _state: &ThemeState) -> Style {
        Style::new().cyan()
    }

    fn info_symbol(&self) -> String {
        "●".to_string()
    }

    fn warning_symbol(&self) -> String {
        "▲".to_string()
    }

    fn error_symbol(&self) -> String {
        "✗".to_string()
    }
}
