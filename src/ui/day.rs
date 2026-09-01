use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use super::widgets::{FOCUSED, SELECTED, UNFOCUSED, styled_event_spans, styled_tags_line};
use crate::{
    app::{App, FocusedPane},
    model::FavoriteLink,
};

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
    let tasks: Vec<_> = app
        .state
        .tasks
        .iter()
        .filter(|t| t.date == Some(app.state.selected_date))
        .collect();
    let capacity = (area.height.saturating_sub(2) / 2).max(1) as usize;
    let start = app
        .state
        .selected_event
        .saturating_sub(capacity.saturating_sub(1));

    let mut all_items = Vec::new();
    for (index, event) in events.iter().enumerate() {
        let is_selected = focused && index == app.state.selected_event;
        let mut lines = vec![Line::from(styled_event_spans(app, event, is_selected))];
        if !event.tags.is_empty() {
            lines.push(styled_tags_line(event, is_selected));
        }
        for link in &event.favorite_links {
            lines.push(styled_favorite_link_line(link, is_selected));
        }
        all_items.push(ListItem::new(lines));
    }
    for (k, task) in tasks.iter().enumerate() {
        let idx = events.len() + k;
        let is_selected = focused && idx == app.state.selected_event;
        let checkbox = if task.is_done {
            Span::styled(
                "[x] ",
                Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled("[ ] ", Style::new().fg(Color::Yellow))
        };
        let imp_symbol = app.config.importance_symbol(task.importance);
        let imp_span = Span::styled(
            format!("{imp_symbol} "),
            super::month::month_importance_style(task.importance),
        );
        let title_style = if is_selected {
            SELECTED
        } else if task.is_done {
            Style::new().fg(Color::DarkGray)
        } else {
            Style::new().fg(Color::White)
        };
        let title_span = Span::styled(&task.title, title_style);
        let mut spans = vec![checkbox, imp_span, title_span];
        if let Some(desc) = &task.description {
            spans.push(Span::styled(
                format!(" ({desc})"),
                Style::new().fg(Color::DarkGray),
            ));
        }
        all_items.push(ListItem::new(vec![Line::from(spans)]));
    }

    let items = all_items
        .into_iter()
        .skip(start)
        .take(capacity)
        .collect::<Vec<_>>();

    let title = if tasks.is_empty() {
        let total_count = events.len();
        if focused {
            format!(" СОБЫТИЯ ({total_count}) ")
        } else {
            format!(" [ СОБЫТИЯ ] ({total_count}) ")
        }
    } else {
        let ev_count = events.len();
        let task_count = tasks.len();
        if focused {
            format!(" СОБЫТИЯ ({ev_count}) │ ЗАДАНИЯ ({task_count}) ")
        } else {
            format!(" [ СОБЫТИЯ ({ev_count}) │ ЗАДАНИЯ ({task_count}) ] ")
        }
    };

    let title_style = if focused {
        SELECTED
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

fn styled_favorite_link_line(link: &FavoriteLink, is_selected: bool) -> Line<'static> {
    let mut spans = vec![
        Span::raw("    "),
        Span::styled(
            format!("🔗 {}", link.label),
            Style::new()
                .fg(if is_selected {
                    Color::Cyan
                } else {
                    Color::White
                })
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if !link.url.is_empty() {
        spans.push(Span::styled(
            format!(" › {}", link.url),
            Style::new().fg(Color::Cyan),
        ));
    }
    Line::from(spans)
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
        format!(" ЗАМЕТКИ ({total_notes}) ")
    } else {
        format!(" [ ЗАМЕТКИ ] ({total_notes}) ")
    };
    let notes_title_style = if notes_focused {
        SELECTED
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
    let selected_event = app.selected_event();
    let event_mode = app.state.focused_pane == FocusedPane::Events
        || (total_notes == 0 && selected_event.is_some());

    let (body_title, body_text) = if event_mode && let Some(event) = selected_event {
        (
            format!(" [ ОПИСАНИЕ: {} ] ", event.title),
            event.description.as_deref().unwrap_or(""),
        )
    } else {
        (
            selected_note
                .and_then(|note| note.title.as_deref())
                .map(|t| format!(" [ ТЕКСТ: {t} ] "))
                .unwrap_or_else(|| " [ ТЕКСТ ЗАМЕТКИ ] ".into()),
            selected_note.map_or("", |note| note.body.as_str()),
        )
    };

    frame.render_widget(
        Paragraph::new(body_text).wrap(Wrap { trim: false }).block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(body_title, Style::new().fg(Color::DarkGray)))
                .border_style(UNFOCUSED),
        ),
        rows[1],
    );

    let links_focused = app.state.focused_pane == FocusedPane::Links;
    let link_capacity = rows[2].height.saturating_sub(2).max(1) as usize;
    let link_start = app
        .state
        .selected_link
        .saturating_sub(link_capacity.saturating_sub(1));

    let (total_links, links): (usize, Vec<ListItem>) =
        if event_mode && let Some(event) = selected_event {
            let total = event.favorite_links.len();
            let items = event
                .favorite_links
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
                .collect();
            (total, items)
        } else {
            let total = selected_note.map_or(0, |note| note.links.len());
            let items = selected_note
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
            (total, items)
        };

    let links_title = if links_focused {
        format!(" ССЫЛКИ ({total_links}) ")
    } else {
        format!(" [ ССЫЛКИ ] ({total_links}) ")
    };
    let links_title_style = if links_focused {
        SELECTED
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FavoriteLink;

    #[test]
    fn favorite_link_line_shows_attached_link() {
        let link = FavoriteLink {
            id: 1,
            label: "Условие".into(),
            url: "https://example.com/task".into(),
            description: None,
            tags: "#дз".into(),
        };

        let line = styled_favorite_link_line(&link, false);
        let rendered = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert_eq!(rendered, "    🔗 Условие › https://example.com/task");
    }

    #[test]
    fn day_view_renders_event_description_and_links_in_details_pane() {
        use chrono::NaiveDate;
        use ratatui::{Terminal, backend::TestBackend};

        use crate::{
            config::Config,
            model::{Importance, NewEvent, NewFavoriteLink},
            storage::Database,
        };

        let mut db = Database::in_memory().unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let link_id = db
            .create_favorite_link(&NewFavoriteLink {
                label: "ГО ссылка".into(),
                url: "https://telemost.yandex.ru/test".into(),
                description: None,
                tags: String::new(),
            })
            .unwrap();

        db.create_event(
            &NewEvent {
                title: "ГО лекция".into(),
                description: Some("Описание лекции".into()),
                start_date: date,
                start_time: None,
                end_time: None,
                importance: Importance::Normal,
                directory: None,
            },
            None,
            &["лекция".into()],
            &[link_id],
        )
        .unwrap();

        let mut app = App::new(db, Config::default()).unwrap();
        app.state.selected_date = date;
        app.state.active_view = crate::app::View::Day;
        app.state.focused_pane = FocusedPane::Events;
        app.refresh_after_change().unwrap();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_day(frame, frame.area(), &app))
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }

        assert!(text.contains("ГО лекция"));
        assert!(text.contains("#лекция"));
        assert!(text.contains("ГО ссылка"));
        assert!(text.contains("ОПИСАНИЕ: ГО лекция"));
        assert!(text.contains("Описание лекции"));
        assert!(text.contains("[ ССЫЛКИ ] (1)"));
    }

    #[test]
    fn day_view_selected_url_returns_event_favorite_link() {
        use chrono::NaiveDate;

        use crate::{
            config::Config,
            model::{Importance, NewEvent, NewFavoriteLink},
            storage::Database,
        };

        let mut db = Database::in_memory().unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let link_id = db
            .create_favorite_link(&NewFavoriteLink {
                label: "ГО ссылка".into(),
                url: "https://telemost.yandex.ru/test".into(),
                description: None,
                tags: String::new(),
            })
            .unwrap();

        db.create_event(
            &NewEvent {
                title: "ГО лекция".into(),
                description: None,
                start_date: date,
                start_time: None,
                end_time: None,
                importance: Importance::Normal,
                directory: None,
            },
            None,
            &[],
            &[link_id],
        )
        .unwrap();

        let mut app = App::new(db, Config::default()).unwrap();
        app.state.selected_date = date;
        app.state.active_view = crate::app::View::Day;
        app.state.focused_pane = FocusedPane::Events;
        app.refresh_after_change().unwrap();

        assert_eq!(
            app.selected_url().as_deref(),
            Some("https://telemost.yandex.ru/test")
        );

        app.state.focused_pane = FocusedPane::Links;
        assert_eq!(
            app.selected_url().as_deref(),
            Some("https://telemost.yandex.ru/test")
        );
    }
}
