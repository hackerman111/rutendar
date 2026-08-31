use chrono::{Datelike, Duration, NaiveDate};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Wrap},
};

use crate::{
    app::{App, Editor, FocusedPane, InputMode, Overlay, Popup, View},
    calendar::{month_start, week_end, week_start},
    model::EventOccurrence,
    search::SearchResult,
};

mod overlay;
use overlay::{render_agenda, render_popup, render_upcoming};

const SELECTED: Style = Style::new().fg(Color::Black).bg(Color::Yellow);
const TODAY: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const FOCUSED: Style = Style::new().fg(Color::Yellow);

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);
    render_header(frame, rows[0], app);
    match app.state.active_view {
        View::Week => render_week(frame, rows[1], app),
        View::Day => render_day(frame, rows[1], app),
        View::Month => render_month(frame, rows[1], app),
        View::Year => render_year(frame, rows[1], app),
    }
    render_next(frame, rows[2], app);
    render_status(frame, rows[3], app);

    match app.state.overlay {
        Some(Overlay::Agenda) => render_agenda(frame, area, app),
        Some(Overlay::Upcoming) => render_upcoming(frame, area, app),
        None => {}
    }
    if let Some(popup) = &app.state.popup {
        render_popup(frame, area, app, popup);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let title = match app.state.active_view {
        View::Week => {
            let range = format!(
                "{} — {}",
                week_start(app.state.selected_date).format("%d.%m.%Y"),
                week_end(app.state.selected_date).format("%d.%m.%Y")
            );
            if app.config.ui.show_week_numbers {
                format!(
                    "W{:02} · {range}",
                    app.state.selected_date.iso_week().week()
                )
            } else {
                range
            }
        }
        View::Day => format!(
            "{} {}{}",
            weekday_long(app.state.selected_date),
            app.state.selected_date.format("%d.%m.%Y"),
            if app.state.selected_date == app.state.today {
                " · TODAY"
            } else {
                ""
            }
        ),
        View::Month => format!(
            "{} {}",
            month_name(app.state.selected_date.month()),
            app.state.selected_date.year()
        ),
        View::Year => app.state.selected_date.year().to_string(),
    };
    frame.render_widget(
        Paragraph::new(title)
            .alignment(Alignment::Center)
            .style(Style::new().add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL)),
        area,
    );
}

fn render_week(frame: &mut Frame, area: Rect, app: &App) {
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

fn render_day(frame: &mut Frame, area: Rect, app: &App) {
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

fn render_day_events(frame: &mut Frame, area: Rect, app: &App) {
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

fn render_day_notes(frame: &mut Frame, area: Rect, app: &App) {
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

fn render_month(frame: &mut Frame, area: Rect, app: &App) {
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

fn render_year(frame: &mut Frame, area: Rect, app: &App) {
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

fn render_next(frame: &mut Frame, area: Rect, app: &App) {
    let mut text = String::from("NEXT  ");
    let mut shown = 0;
    for event in &app.state.next {
        if shown >= app.config.agenda.next_events {
            break;
        }
        let part = relative_event(app, event);
        let separator = if shown == 0 { "" } else { " · " };
        let reserve = 8;
        if text.chars().count() + separator.chars().count() + part.chars().count() + reserve
            > area.width as usize
        {
            break;
        }
        text.push_str(separator);
        text.push_str(&part);
        shown += 1;
    }
    let remaining = app.state.next_total.saturating_sub(shown);
    if remaining > 0 {
        text.push_str(&format!(" · +{remaining}"));
    }
    frame.render_widget(
        Paragraph::new(text).style(Style::new().fg(Color::Cyan)),
        area,
    );
}

fn render_status(frame: &mut Frame, area: Rect, app: &App) {
    let mode = match app.state.input_mode {
        InputMode::Normal => "NORMAL",
        InputMode::Editor => "EDIT",
        InputMode::Search => "SEARCH",
        InputMode::Confirm => "CONFIRM",
        InputMode::Scope => "SCOPE",
        InputMode::GotoDate => "DATE",
    };
    let mut text = format!(
        " {mode} │ {} │ {} │ a events │ t upcoming │ ? help ",
        app.state.active_view.label(),
        app.state.selected_date.format("%d.%m.%Y")
    );
    if let Some(status) = &app.state.status_message {
        text.push_str("│ ");
        text.push_str(status);
    }
    frame.render_widget(
        Paragraph::new(text).style(Style::new().bg(Color::DarkGray)),
        area,
    );
}

fn event_line(app: &App, event: &EventOccurrence) -> String {
    let recurring = if event.is_recurring { "↻" } else { "" };
    let time = event
        .start_time
        .map(|time| time.format("%H:%M").to_string())
        .unwrap_or_else(|| "весь день".into());
    format!(
        "{recurring}{} {time} {}",
        app.config.importance_symbol(event.importance),
        event.title
    )
}

fn tags_line(event: &EventOccurrence) -> String {
    event
        .tags
        .iter()
        .map(|tag| format!("#{}", tag.name))
        .collect::<Vec<_>>()
        .join(" ")
}

fn relative_event(app: &App, event: &EventOccurrence) -> String {
    let date = relative_date(app.state.today, event.date);
    let time = event
        .start_time
        .map(|time| time.format("%H:%M ").to_string())
        .unwrap_or_default();
    format!(
        "{date}{time}{}{}",
        app.config.importance_symbol(event.importance),
        event.title
    )
}

fn relative_date(today: NaiveDate, date: NaiveDate) -> String {
    if date == today {
        String::new()
    } else if date == today + Duration::days(1) {
        "завтра ".into()
    } else {
        format!("{} ", date.format("%d.%m"))
    }
}

fn centered(area: Rect, width_percent: u16, height_percent: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height_percent) / 2),
            Constraint::Percentage(height_percent),
            Constraint::Percentage((100 - height_percent) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_percent) / 2),
            Constraint::Percentage(width_percent),
            Constraint::Percentage((100 - width_percent) / 2),
        ])
        .split(vertical[1])[1]
}

fn centered_fixed(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

fn weekday_short(date: NaiveDate) -> &'static str {
    match date.weekday() {
        chrono::Weekday::Mon => "ПН",
        chrono::Weekday::Tue => "ВТ",
        chrono::Weekday::Wed => "СР",
        chrono::Weekday::Thu => "ЧТ",
        chrono::Weekday::Fri => "ПТ",
        chrono::Weekday::Sat => "СБ",
        chrono::Weekday::Sun => "ВС",
    }
}

fn weekday_long(date: NaiveDate) -> &'static str {
    match date.weekday() {
        chrono::Weekday::Mon => "ПОНЕДЕЛЬНИК",
        chrono::Weekday::Tue => "ВТОРНИК",
        chrono::Weekday::Wed => "СРЕДА",
        chrono::Weekday::Thu => "ЧЕТВЕРГ",
        chrono::Weekday::Fri => "ПЯТНИЦА",
        chrono::Weekday::Sat => "СУББОТА",
        chrono::Weekday::Sun => "ВОСКРЕСЕНЬЕ",
    }
}

fn month_name(month: u32) -> &'static str {
    match month {
        1 => "ЯНВАРЬ",
        2 => "ФЕВРАЛЬ",
        3 => "МАРТ",
        4 => "АПРЕЛЬ",
        5 => "МАЙ",
        6 => "ИЮНЬ",
        7 => "ИЮЛЬ",
        8 => "АВГУСТ",
        9 => "СЕНТЯБРЬ",
        10 => "ОКТЯБРЬ",
        11 => "НОЯБРЬ",
        12 => "ДЕКАБРЬ",
        _ => "",
    }
}
