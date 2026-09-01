use chrono::{Datelike, Duration};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::{
    day::render_day,
    widgets::{FOCUSED, SELECTED, TODAY, event_line, tags_line, weekday_short},
};
use crate::{app::App, calendar::week_start};

pub fn render_week(frame: &mut Frame, area: Rect, app: &App) {
    if area.width < 70 {
        render_day(frame, area, app);
        return;
    }
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 7); 7])
        .split(area);
    let start = week_start(app.state.selected_date);
    for (offset, column) in columns.iter().enumerate() {
        let date = start + Duration::days(offset as i64);
        let events: Vec<_> = app
            .state
            .occurrences
            .iter()
            .filter(|event| event.date == date)
            .collect();
        let mut title = format!("{} {:02}", weekday_short(date), date.day());
        if date == app.state.today {
            title.push_str(" TODAY");
        }
        let selected = date == app.state.selected_date;
        let show_tags = column.width > 15
            && events.len().saturating_mul(2) <= column.height.saturating_sub(2) as usize;
        let lines_per_event = if show_tags { 2 } else { 1 };
        let capacity = (column.height.saturating_sub(2) as usize / lines_per_event).max(1);
        let start = if selected {
            app.state
                .selected_event
                .saturating_sub(capacity.saturating_sub(1))
        } else {
            0
        };
        let lines = events
            .iter()
            .enumerate()
            .skip(start)
            .take(capacity)
            .flat_map(|(index, event)| {
                let mut result = vec![Line::from(event_line(app, event))];
                if show_tags && !event.tags.is_empty() {
                    result.push(Line::from(tags_line(event)));
                }
                if selected && index == app.state.selected_event {
                    result[0] = result[0].clone().style(SELECTED);
                }
                result
            })
            .collect::<Vec<_>>();
        let border_style = if selected { FOCUSED } else { Style::default() };
        let title_style = if date == app.state.today {
            TODAY
        } else {
            Style::default()
        };
        frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: true }).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(title, title_style))
                    .border_style(border_style),
            ),
            *column,
        );
    }
}
