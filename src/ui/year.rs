use chrono::Datelike;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::Span,
    widgets::{Block, Borders, Paragraph},
};

use super::widgets::{FOCUSED, TODAY, month_name};
use crate::app::App;

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
            let events = app
                .state
                .occurrences
                .iter()
                .filter(|event| event.date.month() == month)
                .count();
            let notes = app
                .state
                .notes
                .iter()
                .filter(|note| note.date.month() == month)
                .count();
            let selected = app.state.selected_date.month() == month;
            let title_style = if app.state.today.year() == app.state.selected_date.year()
                && app.state.today.month() == month
            {
                TODAY
            } else {
                Style::default()
            };
            frame.render_widget(
                Paragraph::new(format!("Событий: {events}\nЗаметок: {notes}"))
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(Span::styled(month_name(month), title_style))
                            .border_style(if selected { FOCUSED } else { Style::default() }),
                    ),
                columns[column],
            );
        }
    }
}
