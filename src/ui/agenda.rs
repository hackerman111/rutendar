use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
};

use super::widgets::{SELECTED, centered};
use crate::{app::App, search::SearchResult};

pub fn render_agenda(frame: &mut Frame, area: Rect, app: &App) {
    let popup = centered(area, 92, 86);
    frame.render_widget(Clear, popup);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(3),
        ])
        .split(popup);
    frame.render_widget(
        Paragraph::new(format!("/ {}", app.state.agenda.query))
            .style(if app.state.agenda.searching {
                SELECTED
            } else {
                Style::default()
            })
            .block(Block::default().borders(Borders::ALL).title(" AGENDA ")),
        rows[0],
    );
    let filters = &app.state.agenda.filters;
    let tag_capacity = (popup.width / 12).max(1) as usize;
    let tag_start = app
        .state
        .agenda
        .tag_cursor
        .saturating_sub(tag_capacity.saturating_sub(1));
    let tag_filters = app
        .state
        .agenda
        .available_tags
        .iter()
        .enumerate()
        .skip(tag_start)
        .take(tag_capacity)
        .map(|(index, tag)| {
            let selected = filters.tags.contains(&tag.normalized_name);
            let label = format!("{}#{}", if selected { "✓" } else { "" }, tag.name);
            if index == app.state.agenda.tag_cursor {
                format!("[{label}]")
            } else {
                label
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(format!(
                "f date:{:?}  r type:{:?}  i importance:{:?}  s sort:{:?}  A tags:{:?}",
                filters.date,
                filters.item_type,
                filters.importance,
                filters.sort,
                filters.tag_matching
            )),
            Line::from(format!("[/] tag, Space toggle: {tag_filters}")),
        ]),
        rows[1],
    );
    let capacity = rows[2].height.saturating_sub(3).max(1) as usize;
    let start = app
        .state
        .agenda
        .selected
        .saturating_sub(capacity.saturating_sub(1));
    let table_rows = app
        .state
        .agenda
        .items
        .iter()
        .enumerate()
        .skip(start)
        .take(capacity)
        .map(|(index, item)| {
            let (time, importance, kind, title, tags) = match item {
                SearchResult::Event(event) => (
                    event
                        .start_time
                        .map(|time| time.format("%H:%M").to_string())
                        .unwrap_or_else(|| "весь день".into()),
                    app.config.importance_symbol(event.importance).to_owned(),
                    if event.is_recurring { "↻" } else { "event" }.to_owned(),
                    event.title.clone(),
                    event
                        .tags
                        .iter()
                        .map(|tag| tag.name.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                ),
                SearchResult::Note(note) => (
                    String::new(),
                    String::new(),
                    "note".to_owned(),
                    note.title.clone().unwrap_or_else(|| "Без названия".into()),
                    String::new(),
                ),
            };
            Row::new([
                Cell::from(item.date().format("%d.%m.%Y").to_string()),
                Cell::from(time),
                Cell::from(importance),
                Cell::from(kind),
                Cell::from(title),
                Cell::from(tags),
            ])
            .style(if index == app.state.agenda.selected {
                SELECTED
            } else {
                Style::default()
            })
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Table::new(
            table_rows,
            [
                Constraint::Length(11),
                Constraint::Length(9),
                Constraint::Length(3),
                Constraint::Length(7),
                Constraint::Percentage(38),
                Constraint::Percentage(28),
            ],
        )
        .header(
            Row::new(["DATE", "TIME", "PRI", "TYPE", "EVENT / NOTE", "TAGS"])
                .style(Style::new().add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().borders(Borders::ALL)),
        rows[2],
    );
}
