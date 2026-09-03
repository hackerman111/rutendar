use std::{error::Error, io};

use chrono::{Duration, Local, NaiveDate};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal, TerminalOptions, Viewport,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::format::format_event_card;
use crate::{
    model::{EventOccurrence, Importance},
    storage::Database,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    Day,
    Week,
    Month,
}

impl Period {
    pub fn next(self) -> Self {
        match self {
            Self::Day => Self::Week,
            Self::Week => Self::Month,
            Self::Month => Self::Day,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Day => Self::Month,
            Self::Week => Self::Day,
            Self::Month => Self::Week,
        }
    }

    pub fn date_range(self, today: NaiveDate) -> (NaiveDate, NaiveDate) {
        match self {
            Self::Day => (today, today),
            Self::Week => (today, today + Duration::days(7)),
            Self::Month => (today, today + Duration::days(30)),
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "day" | "d" | "день" => Some(Self::Day),
            "week" | "w" | "неделя" => Some(Self::Week),
            "month" | "m" | "месяц" => Some(Self::Month),
            _ => None,
        }
    }
}

pub fn filter_events<'a>(events: &'a [EventOccurrence], query: &str) -> Vec<&'a EventOccurrence> {
    let clean = query.trim().to_lowercase();
    if clean.is_empty() {
        return events.iter().collect();
    }

    let terms: Vec<&str> = clean.split_whitespace().collect();
    events
        .iter()
        .filter(|event| {
            let title = event.title.to_lowercase();
            let desc = event.description.as_deref().unwrap_or("").to_lowercase();
            let date_str = event.date.format("%d.%m.%Y").to_string();

            terms.iter().all(|term| {
                title.contains(term)
                    || desc.contains(term)
                    || date_str.contains(term)
                    || event.tags.iter().any(|tag| {
                        tag.name
                            .to_lowercase()
                            .contains(term.trim_start_matches('#'))
                    })
            })
        })
        .collect()
}

pub fn run_list(database: &Database, initial_period: Option<Period>) -> Result<(), Box<dyn Error>> {
    let today = Local::now().date_naive();
    let mut period = initial_period.unwrap_or(Period::Week);
    let mut query = String::new();
    let mut selected = 0usize;

    let (start, end) = period.date_range(today);
    let mut all_events = database.events_between(start, end)?;

    enable_raw_mode()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(9),
        },
    )?;

    let selected_event = loop {
        let filtered = filter_events(&all_events, &query);
        if !filtered.is_empty() && selected >= filtered.len() {
            selected = filtered.len() - 1;
        }

        terminal.draw(|frame| {
            let area = frame.area();
            render_inline_menu(frame, area, period, &query, &filtered, selected);
        })?;

        if let Event::Key(key) = event::read()? {
            match (key.code, key.modifiers) {
                (KeyCode::Char('c'), KeyModifiers::CONTROL) | (KeyCode::Esc, _) => {
                    break None;
                }
                (KeyCode::Enter, _) if !filtered.is_empty() => {
                    break Some(filtered[selected].clone());
                }
                (KeyCode::Tab, KeyModifiers::NONE) | (KeyCode::Right, _) => {
                    period = period.next();
                    let (s, e) = period.date_range(today);
                    if let Ok(ev) = database.events_between(s, e) {
                        all_events = ev;
                    }
                    selected = 0;
                }
                (KeyCode::BackTab, _) | (KeyCode::Left, _) => {
                    period = period.prev();
                    let (s, e) = period.date_range(today);
                    if let Ok(ev) = database.events_between(s, e) {
                        all_events = ev;
                    }
                    selected = 0;
                }
                (KeyCode::Up, _)
                | (KeyCode::Char('p'), KeyModifiers::CONTROL)
                | (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                    selected = selected.saturating_sub(1);
                }
                (KeyCode::Down, _)
                | (KeyCode::Char('n'), KeyModifiers::CONTROL)
                | (KeyCode::Char('j'), KeyModifiers::CONTROL)
                    if !filtered.is_empty() && selected + 1 < filtered.len() =>
                {
                    selected += 1;
                }
                (KeyCode::Backspace, _) => {
                    query.pop();
                    selected = 0;
                }
                (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                    query.push(c);
                    selected = 0;
                }
                _ => {}
            }
        }
    };

    terminal.clear()?;
    disable_raw_mode()?;

    if let Some(event) = selected_event {
        println!("{}", format_event_card(&event, crate::ui::Theme::default()));
    }

    Ok(())
}

fn render_inline_menu(
    frame: &mut ratatui::Frame,
    area: Rect,
    period: Period,
    query: &str,
    events: &[&EventOccurrence],
    selected: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(area);

    let active_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let inactive_style = Style::default().fg(Color::DarkGray);

    let tab_day = if period == Period::Day {
        Span::styled(" [ ДЕНЬ ] ", active_style)
    } else {
        Span::styled("  ДЕНЬ  ", inactive_style)
    };
    let tab_week = if period == Period::Week {
        Span::styled(" [ НЕДЕЛЯ ] ", active_style)
    } else {
        Span::styled("  НЕДЕЛЯ  ", inactive_style)
    };
    let tab_month = if period == Period::Month {
        Span::styled(" [ МЕСЯЦ ] ", active_style)
    } else {
        Span::styled("  МЕСЯЦ  ", inactive_style)
    };

    let header_line = Line::from(vec![
        Span::raw("Период: "),
        tab_day,
        Span::raw(" "),
        tab_week,
        Span::raw(" "),
        tab_month,
        Span::styled(
            "  (Tab: период · ↑/↓: выбор · Enter: вывод · Esc: отмена)",
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(header_line), chunks[0]);

    let search_line = Line::from(vec![
        Span::styled("🔍 Поиск: ", Style::default().fg(Color::Yellow)),
        Span::styled(
            if query.is_empty() {
                "начните ввод для фильтрации..."
            } else {
                query
            },
            if query.is_empty() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            },
        ),
        Span::styled("█", Style::default().fg(Color::Cyan)),
        Span::styled(
            format!("  ({} событий)", events.len()),
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(search_line), chunks[1]);

    let list_height = chunks[2].height as usize;
    let scroll_offset = if selected >= list_height {
        selected - list_height + 1
    } else {
        0
    };

    let mut event_lines = Vec::new();
    if events.is_empty() {
        event_lines.push(Line::from(Span::styled(
            "   (нет событий за выбранный период)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (i, event) in events
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(list_height)
        {
            let is_sel = i == selected;
            let marker = if is_sel {
                Span::styled(
                    "▸ ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw("  ")
            };

            let imp_indicator = match event.importance {
                Importance::High => Span::styled(
                    "! ",
                    Style::default()
                        .fg(Color::LightRed)
                        .add_modifier(Modifier::BOLD),
                ),
                Importance::Normal => Span::styled("• ", Style::default().fg(Color::Cyan)),
                Importance::Low => Span::styled("· ", Style::default().fg(Color::Blue)),
                Importance::None => Span::raw("  "),
            };

            let time_str = match (event.start_time, event.end_time) {
                (Some(s), Some(e)) => format!("{}-{} ", s.format("%H:%M"), e.format("%H:%M")),
                (Some(s), None) => format!("{} ", s.format("%H:%M")),
                _ => String::new(),
            };
            let time_span = Span::styled(time_str, Style::default().fg(Color::Yellow));

            let title_style = if is_sel {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let title_span = Span::styled(format!("{} ", event.title), title_style);

            let date_span = Span::styled(
                format!("({}) ", event.date.format("%d.%m.%Y")),
                Style::default().fg(Color::DarkGray),
            );

            let tags_str = event
                .tags
                .iter()
                .map(|t| format!("#{}", t.name))
                .collect::<Vec<_>>()
                .join(" ");
            let tags_span = Span::styled(tags_str, Style::default().fg(Color::Cyan));

            let line_spans = vec![
                marker,
                imp_indicator,
                time_span,
                title_span,
                date_span,
                tags_span,
            ];

            if is_sel {
                event_lines.push(
                    Line::from(line_spans).style(Style::default().bg(Color::Rgb(20, 30, 45))),
                );
            } else {
                event_lines.push(Line::from(line_spans));
            }
        }
    }

    frame.render_widget(
        Paragraph::new(event_lines).block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray)),
        ),
        chunks[2],
    );
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveTime};

    use super::*;
    use crate::model::{Importance, Tag};

    fn make_test_event(title: &str, tag_name: &str) -> EventOccurrence {
        EventOccurrence {
            event_id: 1,
            recurrence_id: None,
            original_date: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            date: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            start_time: Some(NaiveTime::from_hms_opt(10, 0, 0).unwrap()),
            end_time: None,
            title: title.into(),
            description: None,
            importance: Importance::Normal,
            tags: vec![Tag {
                id: 1,
                name: tag_name.into(),
                normalized_name: tag_name.into(),
            }],
            favorite_links: Vec::new(),
            directory: None,
            is_recurring: false,
        }
    }

    #[test]
    fn filter_events_matches_title_and_tag() {
        let events = vec![
            make_test_event("Семинар по физике", "физика"),
            make_test_event("Лекция по матанализу", "матан"),
        ];

        let matched = filter_events(&events, "физика");
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].title, "Семинар по физике");

        let matched_tag = filter_events(&events, "#матан");
        assert_eq!(matched_tag.len(), 1);
        assert_eq!(matched_tag[0].title, "Лекция по матанализу");

        let matched_empty = filter_events(&events, "");
        assert_eq!(matched_empty.len(), 2);
    }

    #[test]
    fn period_date_ranges_and_next() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        assert_eq!(Period::Day.date_range(today), (today, today));
        assert_eq!(
            Period::Week.date_range(today),
            (today, today + Duration::days(7))
        );
        assert_eq!(
            Period::Month.date_range(today),
            (today, today + Duration::days(30))
        );

        assert_eq!(Period::Day.next(), Period::Week);
        assert_eq!(Period::Week.next(), Period::Month);
        assert_eq!(Period::Month.next(), Period::Day);
    }
}
