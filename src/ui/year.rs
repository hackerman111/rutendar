use chrono::Datelike;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::widgets::{SELECTED, TODAY, TODAY_BADGE, calendar_border_style, month_name};
use crate::{app::App, model::Importance};

pub fn render_year(frame: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1, 4); 4])
        .split(area);
    for row in 0..4 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 3); 3])
            .split(rows[row]);
        for column in 0..3 {
            let month = (row * 3 + column + 1) as u32;
            let (events, has_high_importance) = app
                .state
                .occurrences
                .iter()
                .filter(|event| event.date.month() == month)
                .fold((0, false), |(count, important), event| {
                    (count + 1, important || event.importance == Importance::High)
                });
            let notes = app
                .state
                .notes
                .iter()
                .filter(|note| note.date.month() == month)
                .count();
            let selected = app.state.selected_date.month() == month;
            let is_today_month = app.state.today.year() == app.state.selected_date.year()
                && app.state.today.month() == month;

            let title_line = if selected {
                Line::from(vec![Span::styled(
                    format!(" {:02} · {} ", month, month_name(month)),
                    SELECTED,
                )])
            } else if is_today_month {
                Line::from(vec![
                    Span::styled(
                        format!(" {:02} · {} ", month, month_name(month)),
                        TODAY_BADGE,
                    ),
                    Span::styled(" TODAY ", TODAY),
                ])
            } else {
                Line::from(vec![Span::styled(
                    format!(" [{:02} · {}] ", month, month_name(month)),
                    Style::new().fg(Color::DarkGray),
                )])
            };

            let lines = vec![
                Line::from(vec![
                    Span::styled("  СОБЫТИЯ  ", Style::new().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{events:>3} "),
                        if events > 0 {
                            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                        } else {
                            Style::new().fg(Color::DarkGray)
                        },
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  ЗАМЕТКИ  ", Style::new().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{notes:>3} "),
                        if notes > 0 {
                            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                        } else {
                            Style::new().fg(Color::DarkGray)
                        },
                    ),
                ]),
            ];

            let border_style = calendar_border_style(selected, is_today_month, has_high_importance);

            frame.render_widget(
                Paragraph::new(lines).alignment(Alignment::Left).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(title_line)
                        .border_style(border_style),
                ),
                columns[column],
            );
        }
    }
}
