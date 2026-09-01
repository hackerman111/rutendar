use chrono::{Datelike, Duration};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

use super::widgets::{SELECTED, TODAY};
use crate::{
    app::App,
    calendar::{month_start, week_start},
};

pub fn render_month(frame: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
        ])
        .split(area);
    let headers = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 7); 7])
        .split(rows[0]);
    for (index, header) in ["ПН", "ВТ", "СР", "ЧТ", "ПТ", "СБ", "ВС"]
        .iter()
        .enumerate()
    {
        frame.render_widget(
            Paragraph::new(*header).alignment(Alignment::Center),
            headers[index],
        );
    }
    let first = week_start(month_start(app.state.selected_date));
    for week in 0..6 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 7); 7])
            .split(rows[week + 1]);
        for day in 0..7 {
            let date = first + Duration::days((week * 7 + day) as i64);
            let event_count = app
                .state
                .occurrences
                .iter()
                .filter(|item| item.date == date)
                .count();
            let note_count = app
                .state
                .notes
                .iter()
                .filter(|item| item.date == date)
                .count();
            let mut lines = vec![Line::from(date.day().to_string())];
            if date == app.state.today {
                lines.push(Line::from("TODAY"));
            }
            if event_count > 0 || note_count > 0 {
                lines.push(Line::from(format!("{}• {}>", event_count, note_count)));
            }
            let selected = date == app.state.selected_date;
            let mut style = if selected { SELECTED } else { Style::default() };
            if date.month() != app.state.selected_date.month() {
                style = style.fg(Color::DarkGray);
            } else if date == app.state.today && !selected {
                style = TODAY;
            }
            frame.render_widget(
                Paragraph::new(lines)
                    .style(style)
                    .block(Block::default().borders(Borders::ALL)),
                columns[day],
            );
        }
    }
}
