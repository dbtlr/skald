use std::io::{self, Write};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal;

use super::color;

/// Result of the carousel interaction.
#[derive(Debug)]
pub enum CarouselResult {
    /// User accepted the message at this index.
    Accept(usize),
    /// User wants to edit the message at this index.
    Edit(usize),
    /// User wants to extend the message at this index with a body.
    Extend(usize),
    /// User wants the extended action menu for this index.
    Menu(usize),
    /// User aborted (Esc or Ctrl+C).
    Abort,
}

/// Display an interactive carousel of commit messages on stderr.
///
/// Returns the user's chosen action. The caller is responsible for
/// acting on the result (committing, opening an editor, etc.).
pub fn show_carousel(messages: &[String]) -> io::Result<CarouselResult> {
    assert!(!messages.is_empty(), "carousel requires at least one message");

    let total = messages.len();
    let mut current: usize = 0;
    let mut first_render = true;

    // Number of lines the carousel occupies (for re-rendering in place).
    let line_count = 6;

    terminal::enable_raw_mode()?;

    // Ensure we always clean up raw mode, even on early return.
    let result = (|| -> io::Result<CarouselResult> {
        loop {
            render(&messages[current], current, total, first_render, line_count)?;
            first_render = false;

            // Block for next key event.
            if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
                match (code, modifiers) {
                    // Abort
                    (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        return Ok(CarouselResult::Abort);
                    }
                    // Accept
                    (KeyCode::Enter, _) | (KeyCode::Char('a'), KeyModifiers::NONE) => {
                        return Ok(CarouselResult::Accept(current));
                    }
                    // Edit
                    (KeyCode::Char('e'), KeyModifiers::NONE) => {
                        return Ok(CarouselResult::Edit(current));
                    }
                    // Extend
                    (KeyCode::Char('x'), KeyModifiers::NONE) => {
                        return Ok(CarouselResult::Extend(current));
                    }
                    // Menu
                    (KeyCode::Char('?'), _) => {
                        return Ok(CarouselResult::Menu(current));
                    }
                    // Navigate left (wraps)
                    (KeyCode::Left, _) => {
                        current = if current == 0 { total - 1 } else { current - 1 };
                    }
                    // Navigate right (wraps)
                    (KeyCode::Right, _) => {
                        current = (current + 1) % total;
                    }
                    _ => {}
                }
            }
        }
    })();

    terminal::disable_raw_mode()?;
    // Print a newline so the next output starts on a fresh line.
    eprint!("\r\n");

    result
}

/// Render one frame of the carousel to stderr.
fn render(
    message: &str,
    index: usize,
    total: usize,
    first_render: bool,
    line_count: usize,
) -> io::Result<()> {
    let mut out = io::stderr().lock();

    // Move cursor up and clear if re-rendering.
    if !first_render {
        write!(out, "\x1b[{line_count}A\x1b[J")?;
    }

    let bar = color::info();
    let dim = color::dim();
    let pipe = bar.apply_to("\u{2502}");

    let header = format!("Suggestion {} of {}", index + 1, total);
    let arrows = if total > 1 { "\u{25c0} \u{25b6}" } else { "" };

    // Line 1: header with arrows
    write!(out, "{pipe}  {header}  {arrows}\r\n")?;
    // Line 2: blank
    write!(out, "{pipe}\r\n")?;
    // Line 3: the commit message
    write!(out, "{pipe}    {message}\r\n")?;
    // Line 4: blank
    write!(out, "{pipe}\r\n")?;
    // Line 5: separator
    let sep = dim.apply_to("\u{2504}".repeat(48));
    write!(out, "{pipe}  {sep}\r\n")?;
    // Line 6: key hints
    let hints = format!(
        "{}  {}  {}  {}  {}",
        dim.apply_to("\u{2190} \u{2192} cycle"),
        dim.apply_to("a accept"),
        dim.apply_to("e edit"),
        dim.apply_to("x extend"),
        dim.apply_to("? more"),
    );
    write!(out, "{pipe}  {hints}\r\n")?;

    out.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carousel_result_variants_exist() {
        let _ = CarouselResult::Accept(0);
        let _ = CarouselResult::Edit(1);
        let _ = CarouselResult::Extend(2);
        let _ = CarouselResult::Menu(3);
        let _ = CarouselResult::Abort;
    }
}
