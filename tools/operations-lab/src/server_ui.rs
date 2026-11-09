//! The server screen: lab Gecko settings, executions, and the event log.

use std::time::Duration;

use operations::{OperationId, Outcome};
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Row as TableRow, Table, TableState, Wrap};

use crate::server::{INTERVALS_MS, Lab};
use crate::tui::{Flow, Screen, hints, log_line};

/// The server screen over a running [`Lab`].
pub struct ServerScreen {
    lab: Lab,
    table: TableState,
}

impl ServerScreen {
    /// A screen over `lab`.
    #[must_use]
    pub fn new(lab: Lab) -> Self {
        Self {
            lab,
            table: TableState::default(),
        }
    }

    /// Gives the lab back, to close it.
    #[must_use]
    pub fn into_lab(self) -> Lab {
        self.lab
    }

    fn selected(&self) -> Option<OperationId> {
        let rows = self.lab.rows();
        let index = self
            .table
            .selected()
            .unwrap_or(rows.len().saturating_sub(1));
        rows.get(index).map(|r| r.operation_id)
    }

    fn settings(&self) -> Vec<Line<'static>> {
        let script = self.lab.script();
        let counters = self.lab.counters();
        let on_off = |on: bool, yes: &str, no: &str| {
            if on {
                Span::styled(
                    yes.to_string(),
                    Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(no.to_string(), Style::new().fg(Color::Green))
            }
        };
        let field = |name: &str| Span::styled(format!("{name:<17}"), Style::new().fg(Color::Gray));
        vec![
            Line::from(Span::styled(
                "Executor (next handoffs)",
                Style::new().add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![field("[a] handoff"), Span::raw(script.ack.label())]),
            Line::from(vec![
                field("[+/-] steps"),
                Span::raw(script.steps.to_string()),
            ]),
            Line::from(vec![
                field("[i] interval"),
                Span::raw(format!("{} ms", script.interval.as_millis())),
            ]),
            Line::from(vec![field("[o] then"), Span::raw(script.finish.label())]),
            Line::from(""),
            Line::from(Span::styled(
                "Faults",
                Style::new().add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                field("[d] reply delay"),
                on_off(
                    !self.lab.reply_delay().is_zero(),
                    &format!("{} ms", self.lab.reply_delay().as_millis()),
                    "0 ms",
                ),
            ]),
            Line::from(vec![
                field("[l] proxy link"),
                on_off(!self.lab.link_up(), "CUT", "up"),
            ]),
            Line::from(vec![
                field("[w] observe"),
                on_off(self.lab.observe_paused(), "PAUSED", "flowing"),
            ]),
            Line::from(vec![field("[c] commit"), Span::raw("inject 1 failure")]),
            Line::from(""),
            Line::from(Span::styled(
                "Core totals",
                Style::new().add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                field("operations"),
                Span::raw(counters.operations.to_string()),
            ]),
            Line::from(vec![
                field("retry keys"),
                Span::raw(counters.mappings.to_string()),
            ]),
            Line::from(vec![
                field("handoffs"),
                Span::raw(counters.handoffs.to_string()),
            ]),
        ]
    }
}

fn state_style(state: &str) -> Style {
    match state {
        "succeeded" => Style::new().fg(Color::Green),
        "failed" => Style::new().fg(Color::Red),
        "unknown" => Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        "pending" => Style::new().fg(Color::Blue),
        _ => Style::new(),
    }
}

impl Screen for ServerScreen {
    fn draw(&mut self, frame: &mut Frame) {
        let [header, body, log_area, footer] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Percentage(45),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .areas(frame.area());
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    " operations-lab server ",
                    Style::new().fg(Color::Black).bg(Color::Green),
                ),
                Span::raw(format!(
                    "  direct {}   via proxy {}   caller {}   lease {} s   observer buffer {}",
                    self.lab.endpoint(),
                    self.lab.proxy_endpoint(),
                    self.lab.config().caller,
                    self.lab.config().observe_lease.as_secs(),
                    self.lab.config().observer_capacity
                )),
            ])),
            header,
        );

        let [left, right] =
            Layout::horizontal([Constraint::Length(40), Constraint::Min(40)]).areas(body);
        frame.render_widget(
            Paragraph::new(self.settings()).block(Block::bordered().title(" Settings ")),
            left,
        );

        let rows = self.lab.rows();
        if self.table.selected().is_none() && !rows.is_empty() {
            self.table.select(Some(rows.len() - 1));
        }
        let table_rows: Vec<TableRow> = rows
            .iter()
            .map(|row| {
                let (state, sequence, stage, result) = row.report.as_ref().map_or_else(
                    || ("?".to_string(), String::new(), String::new(), String::new()),
                    |r| {
                        (
                            format!("{:?}", r.state).to_lowercase(),
                            r.sequence.to_string(),
                            r.progress
                                .as_ref()
                                .map_or_else(String::new, |p| p.stage.clone()),
                            match &r.result {
                                operations::Value::Null => String::new(),
                                operations::Value::Text(t) => t.clone(),
                                other => format!("{other:?}"),
                            },
                        )
                    },
                );
                let style = state_style(&state);
                TableRow::new(vec![
                    row.operation_id.to_string(),
                    state,
                    sequence,
                    row.handoffs.to_string(),
                    row.handoff.to_string(),
                    row.events_sent.to_string(),
                    row.observers.to_string(),
                    stage,
                    result,
                ])
                .style(style)
            })
            .collect();
        let table = Table::new(
            table_rows,
            [
                Constraint::Length(36),
                Constraint::Length(9),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(12),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(10),
                Constraint::Min(10),
            ],
        )
        .header(
            TableRow::new(vec![
                "operation id", "state", "seq", "hnd", "handoff", "ev", "obs", "stage", "result",
            ])
            .style(Style::new().add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        )
        .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .block(Block::bordered().title(format!(
            " Executions ({}) — ↑/↓ select; on the selected: p progress, s succeed, f fail, k ack, r resend last ",
            rows.len()
        )));
        frame.render_stateful_widget(table, right, &mut self.table);

        let height = usize::from(log_area.height.saturating_sub(2));
        let lines: Vec<Line> = self
            .lab
            .log()
            .tail(height)
            .iter()
            .map(|l| log_line(l))
            .collect();
        let title = self.lab.log().path().map_or_else(
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
                    ("a", "handoff"),
                    ("+/-", "steps"),
                    ("i", "interval"),
                    ("o", "then"),
                    ("d", "reply delay"),
                    ("l", "link"),
                    ("w", "observe pause"),
                    ("c", "commit failure"),
                ]),
                hints(&[
                    ("p", "progress"),
                    ("s", "succeed"),
                    ("f", "fail"),
                    ("k", "ack (evidence)"),
                    ("r", "resend last"),
                    ("q", "quit"),
                ]),
            ]),
            footer,
        );
    }

    fn key(&mut self, key: KeyEvent) -> Flow {
        let mut script = self.lab.script();
        match key.code {
            KeyCode::Char('q') => return Flow::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Flow::Quit;
            }
            KeyCode::Char('a') => script.ack = script.ack.next(),
            KeyCode::Char('+' | '=') => script.steps = (script.steps + 1).min(50),
            KeyCode::Char('-') => script.steps = script.steps.saturating_sub(1),
            KeyCode::Char('i') => {
                let current = u64::try_from(script.interval.as_millis()).unwrap_or(0);
                let next = INTERVALS_MS
                    .iter()
                    .position(|&ms| ms == current)
                    .map_or(0, |i| (i + 1) % INTERVALS_MS.len());
                script.interval = Duration::from_millis(INTERVALS_MS[next]);
            }
            KeyCode::Char('o') => script.finish = script.finish.next(),
            KeyCode::Char('d') => self.lab.cycle_reply_delay(),
            KeyCode::Char('l') => self.lab.toggle_link(),
            KeyCode::Char('w') => self.lab.toggle_observe_pause(),
            KeyCode::Char('c') => self.lab.inject_commit_failure(),
            KeyCode::Up => self.table.select_previous(),
            KeyCode::Down => {
                let last = self.lab.rows().len().saturating_sub(1);
                let next = self.table.selected().map_or(0, |i| (i + 1).min(last));
                self.table.select(Some(next));
            }
            KeyCode::Char(c @ ('p' | 's' | 'f' | 'k' | 'r')) => {
                if let Some(id) = self.selected() {
                    match c {
                        'p' => self.lab.send_progress(id),
                        's' => self.lab.send_terminal(id, Outcome::Succeeded),
                        'f' => self.lab.send_terminal(id, Outcome::Failed),
                        'k' => self.lab.send_ack(id),
                        _ => self.lab.resend_last(id),
                    }
                }
            }
            _ => {}
        }
        self.lab.set_script(script);
        Flow::Continue
    }
}
