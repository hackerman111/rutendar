use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem},
};

use super::widgets::{SELECTED, centered, event_line, relative_date, tags_line};
use crate::app::App;

pub fn render_upcoming(frame: &mut Frame, area: Rect, app: &App) {
    let popup = centered(area, 72, 80);
    frame.render_widget(Clear, popup);
    let capacity = (popup.height.saturating_sub(2) / 2).max(1) as usize;
    let start = app
        .state
        .upcoming
        .selected
        .saturating_sub(capacity.saturating_sub(1));
    let items = app
        .state
        .upcoming
        .items
        .iter()
        .enumerate()
        .skip(start)
        .take(capacity)
        .map(|(index, event)| {
            let line = format!(
                "{}  {}",
                relative_date(app.state.today, event.date),
                event_line(app, event)
            );
            let mut details = tags_line(event);
            if let Some(link) = app
                .state
                .upcoming
                .links_by_date
                .get(&event.date)
                .and_then(|links| links.first())
            {
                details.push_str(&format!("  🔗 {}", link.label));
            }
            ListItem::new(vec![Line::from(line), Line::from(details)]).style(
                if index == app.state.upcoming.selected {
                    SELECTED
                } else {
                    Style::default()
                },
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title(format!(
            " БЛИЖАЙШИЕ · s sort:{:?} ",
            app.state.upcoming.sort
        ))),
        popup,
    );
}
