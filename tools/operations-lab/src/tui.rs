//! The terminal loop shared by both screens.

use std::io;
use std::time::Duration;

use ratatui::Frame;
use ratatui::crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyEvent, KeyEventKind,
};
use ratatui::crossterm::execute;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// What a key press asks the loop to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    /// Keep running.
    Continue,
    /// Leave the screen.
    Quit,
}

/// A screen the loop drives.
pub trait Screen {
    /// Draws the whole screen.
    fn draw(&mut self, frame: &mut Frame);
    /// Handles one key press.
    fn key(&mut self, key: KeyEvent) -> Flow;
    /// Handles pasted text.
    fn paste(&mut self, _text: &str) {}
    /// Called about ten times a second, between events.
    fn tick(&mut self) {}
}

/// Runs `screen` until it asks to quit, restoring the terminal afterwards
/// (also on a panic).
///
/// # Errors
///
/// A terminal I/O error.
pub fn run(screen: &mut impl Screen) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let _ = execute!(io::stdout(), EnableBracketedPaste);
    let result = (|| -> io::Result<()> {
        loop {
            terminal.draw(|frame| screen.draw(frame))?;
            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        if screen.key(key) == Flow::Quit {
                            return Ok(());
                        }
                    }
                    Event::Paste(text) => screen.paste(&text),
                    _ => {}
                }
            }
            screen.tick();
        }
    })();
    let _ = execute!(io::stdout(), DisableBracketedPaste);
    ratatui::restore();
    result
}

/// A key hint: the key highlighted, then its meaning.
#[must_use]
pub fn hint(key: &str, meaning: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(
            format!(" {key} "),
            Style::new()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" {meaning}  ")),
    ]
}

/// A line of key hints.
#[must_use]
pub fn hints(pairs: &[(&str, &str)]) -> Line<'static> {
    Line::from(
        pairs
            .iter()
            .flat_map(|(key, meaning)| hint(key, meaning))
            .collect::<Vec<_>>(),
    )
}

/// Colours a log line by its source and by warnings in it.
#[must_use]
pub fn log_line(line: &str) -> Line<'static> {
    let style = if line.contains("!!") || line.contains("VIOLATION") {
        Style::new().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else if line.contains("no answer") || line.contains("rejected") || line.contains("CUT") {
        Style::new().fg(Color::Yellow)
    } else if line.contains("[exec]") {
        Style::new().fg(Color::Magenta)
    } else if line.contains("[lab]") {
        Style::new().fg(Color::Cyan)
    } else if line.contains("<-") {
        Style::new().fg(Color::Green)
    } else {
        Style::new()
    };
    Line::styled(line.to_string(), style)
}
