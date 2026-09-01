use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use super::widgets::{FOCUSED, SELECTED, event_line, tags_line};
use crate::app::{App, FocusedPane};

pub fn render_day(frame: &mut Frame, area: Rect, app: &App) {
    let direction = if area.width >= 80 {
        Direction::Horizontal
    } else {
        Direction::Vertical
    };
    let panes = Layout::default()
        .direction(direction)
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
        .split(area);
    render_day_events(frame, panes[0], app);
    render_day_notes(frame, panes[1], app);
}

pub fn render_day_events(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.state.focused_pane == FocusedPane::Events;
    let capacity = (area.height.saturating_sub(2) / 2).max(1) as usize;
    let start = app
        .state
        .selected_event
        .saturating_sub(capacity.saturating_sub(1));
    let items = app
        .events_on_selected_date()
        .enumerate()
        .skip(start)
        .take(capacity)
        .map(|(index, event)| {
            let mut lines = vec![Line::from(event_line(app, event))];
            if !event.tags.is_empty() {
                lines.push(Line::from(tags_line(event)).style(Style::new().fg(Color::DarkGray)));
            }
            ListItem::new(lines).style(if focused && index == app.state.selected_event {
                SELECTED
            } else {
                Style::default()
            })
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" СОБЫТИЯ ")
                .border_style(if focused { FOCUSED } else { Style::default() }),
        ),
        area,
    );
}

pub fn render_day_notes(frame: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Percentage(35),
            Constraint::Percentage(30),
        ])
        .split(area);
    let notes_focused = app.state.focused_pane == FocusedPane::Notes;
    let note_capacity = rows[0].height.saturating_sub(2).max(1) as usize;
    let note_start = app
        .state
        .selected_note
        .saturating_sub(note_capacity.saturating_sub(1));
    let note_items = app
        .notes_on_selected_date()
        .enumerate()
        .skip(note_start)
        .take(note_capacity)
        .map(|(index, note)| {
            ListItem::new(format!(
                "> {}",
                note.title.as_deref().unwrap_or("Без названия")
            ))
            .style(if notes_focused && index == app.state.selected_note {
                SELECTED
            } else {
                Style::default()
            })
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(note_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" ЗАМЕТКИ ")
                .border_style(if notes_focused {
                    FOCUSED
                } else {
                    Style::default()
                }),
        ),
        rows[0],
    );
    let selected_note = app.selected_note();
    frame.render_widget(
        Paragraph::new(selected_note.map_or("", |note| note.body.as_str()))
            .wrap(Wrap { trim: false })
            .block(
                Block::default().borders(Borders::ALL).title(
                    selected_note
                        .and_then(|note| note.title.as_deref())
                        .unwrap_or(" ЗАМЕТКА "),
                ),
            ),
        rows[1],
    );
    let links_focused = app.state.focused_pane == FocusedPane::Links;
    let link_capacity = rows[2].height.saturating_sub(2).max(1) as usize;
    let link_start = app
        .state
        .selected_link
        .saturating_sub(link_capacity.saturating_sub(1));
    let links = selected_note
        .map(|note| {
            note.links
                .iter()
                .enumerate()
                .skip(link_start)
                .take(link_capacity)
                .map(|(index, link)| {
                    ListItem::new(format!("> {}", link.label)).style(
                        if links_focused && index == app.state.selected_link {
                            SELECTED
                        } else {
                            Style::default()
                        },
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    frame.render_widget(
        List::new(links).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" ССЫЛКИ ")
                .border_style(if links_focused {
                    FOCUSED
                } else {
                    Style::default()
                }),
        ),
        rows[2],
    );
}
