use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use super::widgets::{FOCUSED, UNFOCUSED, styled_event_spans, styled_tags_line};
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
    let events: Vec<_> = app.events_on_selected_date().collect();
    let total_count = events.len();
    let capacity = (area.height.saturating_sub(2) / 2).max(1) as usize;
    let start = app
        .state
        .selected_event
        .saturating_sub(capacity.saturating_sub(1));

    let items = events
        .iter()
        .copied()
        .enumerate()
        .skip(start)
        .take(capacity)
        .map(|(index, event)| {
            let is_selected = focused && index == app.state.selected_event;
            let mut lines = vec![Line::from(styled_event_spans(app, event, is_selected))];
            if !event.tags.is_empty() {
                lines.push(styled_tags_line(event, is_selected));
            }
            ListItem::new(lines)
        })
        .collect::<Vec<_>>();

    let title = if focused {
        format!(" ▌СОБЫТИЯ▐ ({total_count}) ")
    } else {
        format!(" [ СОБЫТИЯ ] ({total_count}) ")
    };

    let title_style = if focused {
        Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(Color::DarkGray)
    };

    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(title, title_style))
                .border_style(if focused { FOCUSED } else { UNFOCUSED }),
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
    let notes: Vec<_> = app.notes_on_selected_date().collect();
    let total_notes = notes.len();
    let note_capacity = rows[0].height.saturating_sub(2).max(1) as usize;
    let note_start = app
        .state
        .selected_note
        .saturating_sub(note_capacity.saturating_sub(1));

    let note_items = notes
        .iter()
        .copied()
        .enumerate()
        .skip(note_start)
        .take(note_capacity)
        .map(|(index, note)| {
            let is_selected = notes_focused && index == app.state.selected_note;
            let title = note.title.as_deref().unwrap_or("Без названия");
            let line = if is_selected {
                Line::from(Span::styled(
                    format!("▸ {title} "),
                    Style::new()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        title,
                        if note.title.is_some() {
                            Style::new().fg(Color::White)
                        } else {
                            Style::new().fg(Color::DarkGray)
                        },
                    ),
                ])
            };
            ListItem::new(line)
        })
        .collect::<Vec<_>>();

    let notes_title = if notes_focused {
        format!(" ▌ЗАМЕТКИ▐ ({total_notes}) ")
    } else {
        format!(" [ ЗАМЕТКИ ] ({total_notes}) ")
    };
    let notes_title_style = if notes_focused {
        Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(Color::DarkGray)
    };

    frame.render_widget(
        List::new(note_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(notes_title, notes_title_style))
                .border_style(if notes_focused { FOCUSED } else { UNFOCUSED }),
        ),
        rows[0],
    );

    let selected_note = app.selected_note();
    let body_title = selected_note
        .and_then(|note| note.title.as_deref())
        .map(|t| format!(" [ ТЕКСТ: {t} ] "))
        .unwrap_or_else(|| " [ ТЕКСТ ЗАМЕТКИ ] ".into());

    frame.render_widget(
        Paragraph::new(selected_note.map_or("", |note| note.body.as_str()))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(body_title, Style::new().fg(Color::DarkGray)))
                    .border_style(UNFOCUSED),
            ),
        rows[1],
    );

    let links_focused = app.state.focused_pane == FocusedPane::Links;
    let total_links = selected_note.map_or(0, |note| note.links.len());
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
                    let is_selected = links_focused && index == app.state.selected_link;
                    let line = if is_selected {
                        Line::from(Span::styled(
                            format!("▸ 🔗 {} › {} ", link.label, link.url),
                            Style::new()
                                .fg(Color::Black)
                                .bg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ))
                    } else {
                        Line::from(vec![
                            Span::raw("  🔗 "),
                            Span::styled(&link.label, Style::new().fg(Color::White)),
                            Span::styled(" › ", Style::new().fg(Color::DarkGray)),
                            Span::styled(&link.url, Style::new().fg(Color::Cyan)),
                        ])
                    };
                    ListItem::new(line)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let links_title = if links_focused {
        format!(" ▌ССЫЛКИ▐ ({total_links}) ")
    } else {
        format!(" [ ССЫЛКИ ] ({total_links}) ")
    };
    let links_title_style = if links_focused {
        Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(Color::DarkGray)
    };

    frame.render_widget(
        List::new(links).block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(links_title, links_title_style))
                .border_style(if links_focused { FOCUSED } else { UNFOCUSED }),
        ),
        rows[2],
    );
}
