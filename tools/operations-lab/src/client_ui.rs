//! The client screen: the request form, the Observe status, and the log.

use std::sync::mpsc::Receiver;

use operations::RequestId;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use crate::client::{Form, LabClient, Notice};
use crate::tui::{Flow, Screen, hints, log_line};

const FIELDS: [&str; 7] = [
    "type",
    "message",
    "repeat",
    "request_id",
    "operation id",
    "after",
    "burst size",
];

/// The client screen over a [`LabClient`].
pub struct ClientScreen {
    client: LabClient,
    notices: Receiver<Notice>,
    form: Form,
    field: usize,
    /// The field's value before editing started, while editing.
    editing: Option<String>,
}

impl ClientScreen {
    /// A screen over `client` and the receiver of its notices.
    #[must_use]
    pub fn new(client: LabClient, notices: Receiver<Notice>) -> Self {
        Self {
            client,
            notices,
            form: Form::default(),
            field: 0,
            editing: None,
        }
    }

    /// Gives the client back, to close it.
    #[must_use]
    pub fn into_client(self) -> LabClient {
        self.client
    }

    fn value(&mut self, index: usize) -> &mut String {
        match index {
            0 => &mut self.form.operation_type,
            1 => &mut self.form.message,
            2 => &mut self.form.repeat,
            3 => &mut self.form.request_id,
            4 => &mut self.form.operation_id,
            5 => &mut self.form.after,
            _ => &mut self.form.burst,
        }
    }

    fn edit_key(&mut self, key: KeyEvent) {
        let field = self.field;
        match key.code {
            KeyCode::Enter => self.editing = None,
            KeyCode::Esc => {
                if let Some(before) = self.editing.take() {
                    *self.value(field) = before;
                }
            }
            KeyCode::Backspace => {
                self.value(field).pop();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.value(field).clear();
            }
            KeyCode::Char(c) => self.value(field).push(c),
            _ => {}
        }
    }

    fn form_lines(&mut self) -> Vec<Line<'static>> {
        (0..FIELDS.len())
            .map(|i| {
                let selected = i == self.field;
                let editing = selected && self.editing.is_some();
                let value = self.value(i).clone();
                let marker = if editing {
                    "✎ "
                } else if selected {
                    "▶ "
                } else {
                    "  "
                };
                let value_style = if editing {
                    Style::new().fg(Color::Black).bg(Color::Yellow)
                } else if selected {
                    Style::new().add_modifier(Modifier::REVERSED)
                } else {
                    Style::new()
                };
                Line::from(vec![
                    Span::raw(marker),
                    Span::styled(format!("{:<13}", FIELDS[i]), Style::new().fg(Color::Gray)),
                    Span::styled(
                        if value.is_empty() && !editing {
                            "—".to_string()
                        } else {
                            value
                        },
                        value_style,
                    ),
                    Span::raw(if editing { "▏" } else { "" }),
                ])
            })
            .collect()
    }

    fn observe_lines(&self) -> Vec<Line<'static>> {
        let status = self.client.observe_status();
        let Some(id) = status.operation_id else {
            return vec![Line::from("not observing; press o")];
        };
        let state = if status.open {
            Span::styled("open", Style::new().fg(Color::Green))
        } else {
            Span::styled(
                format!("ended: {}", status.ended.unwrap_or_default()),
                Style::new().fg(Color::Yellow),
            )
        };
        let violations = if status.violations == 0 {
            Span::styled("0", Style::new().fg(Color::Green))
        } else {
            Span::styled(
                status.violations.to_string(),
                Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
            )
        };
        vec![
            Line::from(format!("operation  {id}")),
            Line::from(vec![Span::raw("stream     "), state]),
            Line::from(format!(
                "held seq   {}   (r resumes after it)",
                status
                    .held
                    .map_or_else(|| "—".to_string(), |s| s.to_string())
            )),
            Line::from(format!("snapshots  {}", status.received)),
            Line::from(vec![Span::raw("violations "), violations]),
        ]
    }
}

impl Screen for ClientScreen {
    fn draw(&mut self, frame: &mut Frame) {
        let [header, body, log_area, footer] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(9),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .areas(frame.area());
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    " operations-lab client ",
                    Style::new().fg(Color::Black).bg(Color::Blue),
                ),
                Span::raw(format!("  endpoint {}   ", self.client.endpoint())),
                if self.client.is_connected() {
                    Span::styled("link up", Style::new().fg(Color::Green))
                } else {
                    Span::styled(
                        "NO LINK (Zenoh reconnects by itself)",
                        Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
                    )
                },
                Span::raw(format!(
                    "   Start/Get timeout {} ms",
                    self.client.timeout().as_millis()
                )),
            ])),
            header,
        );
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)])
                .areas(body);
        let title = if self.editing.is_some() {
            " Request — editing: Enter keep, Esc undo, Ctrl+U clear "
        } else {
            " Request — ↑/↓ field, Enter edit, n new request_id "
        };
        let lines = self.form_lines();
        frame.render_widget(
            Paragraph::new(lines).block(Block::bordered().title(title)),
            left,
        );
        frame.render_widget(
            Paragraph::new(self.observe_lines()).block(Block::bordered().title(" Observe ")),
            right,
        );

        let height = usize::from(log_area.height.saturating_sub(2));
        let lines: Vec<Line> = self
            .client
            .log()
            .tail(height)
            .iter()
            .map(|l| log_line(l))
            .collect();
        let title = self.client.log().path().map_or_else(
            || " Log ".to_string(),
            |p| format!(" Log (also in {}) ", p.display()),
        );
        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .block(Block::bordered().title(title)),
            log_area,
        );
        frame.render_widget(
            Paragraph::new(vec![
                hints(&[
                    ("s", "start"),
                    ("n", "new request_id"),
                    ("b", "burst same"),
                    ("B", "burst changed"),
                    ("m", "malformed"),
                    ("g", "get"),
                ]),
                hints(&[
                    ("o", "observe"),
                    ("r", "resume"),
                    ("x", "drop stream"),
                    ("q", "quit"),
                ]),
            ]),
            footer,
        );
    }

    fn key(&mut self, key: KeyEvent) -> Flow {
        if self.editing.is_some() {
            self.edit_key(key);
            return Flow::Continue;
        }
        match key.code {
            KeyCode::Char('q') => return Flow::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Flow::Quit;
            }
            KeyCode::Up => self.field = self.field.saturating_sub(1),
            KeyCode::Down | KeyCode::Tab => self.field = (self.field + 1) % FIELDS.len(),
            KeyCode::Enter => self.editing = Some(self.value(self.field).clone()),
            KeyCode::Char('n') => {
                self.form.request_id = RequestId::generate().to_string();
                self.client
                    .log()
                    .push(format!("[client] new request_id {}", self.form.request_id));
            }
            KeyCode::Char('s') => self.client.start(&self.form),
            KeyCode::Char('b') => self.client.burst(&self.form, false),
            KeyCode::Char('B') => self.client.burst(&self.form, true),
            KeyCode::Char('m') => self.client.start_malformed(),
            KeyCode::Char('g') => self.client.get(&self.form),
            KeyCode::Char('o') => self.client.observe(&self.form),
            KeyCode::Char('r') => self.client.resume(),
            KeyCode::Char('x') => self.client.stop_observe(),
            _ => {}
        }
        Flow::Continue
    }

    fn paste(&mut self, text: &str) {
        let text = text.trim();
        let field = self.field;
        if self.editing.is_none() {
            // Pasting replaces the selected field.
            text.clone_into(self.value(field));
        } else {
            self.value(field).push_str(text);
        }
    }

    fn tick(&mut self) {
        while let Ok(notice) = self.notices.try_recv() {
            match notice {
                Notice::Accepted(id) => self.form.operation_id = id.to_string(),
            }
        }
    }
}
