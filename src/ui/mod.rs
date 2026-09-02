pub mod agenda;
pub mod day;
pub mod link_bank;
pub mod month;
pub mod popup;
pub mod theme;
pub mod upcoming;
pub mod week;
pub mod widgets;
pub mod year;

pub use theme::Theme;

use chrono::Datelike;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use self::{
    agenda::render_agenda,
    day::render_day,
    month::render_month,
    popup::render_popup,
    upcoming::render_upcoming,
    week::render_week,
    widgets::{month_name, weekday_long},
    year::render_year,
};

use crate::{
    app::{App, InputMode, Overlay, View},
    calendar::{week_end, week_start},
};

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
    let theme = app.config.ui.theme;
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

    let mut spans = vec![
        Span::styled(" RUTENDAR ", theme.active_tab_style()),
        Span::raw(" "),
    ];

    // View tabs
    for view in [View::Week, View::Day, View::Month, View::Year] {
        if app.state.active_view == view {
            spans.push(Span::styled(
                format!(" {} ", view.label()),
                widgets::theme_selected(theme),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {} ", view.label()),
                widgets::theme_unfocused(theme),
            ));
        }
    }

    let sep = if theme == Theme::Ascii {
        " | "
    } else {
        " │ "
    };

    spans.push(Span::styled(sep, widgets::theme_unfocused(theme)));
    spans.push(Span::styled(title, theme.title_style(true, false)));

    // Right-aligned today indicator + Theme badge if width permits
    let theme_str = format!("[F5: {}] ", theme.name());
    let today_str = format!(
        "{} TODAY: {} ",
        theme.date_icon(),
        app.state.today.format("%d.%m")
    );
    let right_info = format!("{theme_str}{today_str}");
    let current_len: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    if (area.width as usize).saturating_sub(4) > current_len + right_info.len() {
        let padding = (area.width as usize).saturating_sub(4) - current_len - right_info.len();
        spans.push(Span::raw(" ".repeat(padding)));
        spans.push(Span::styled(theme_str, theme.key_badge_style()));
        spans.push(Span::styled(today_str, widgets::theme_unfocused(theme)));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .alignment(Alignment::Left)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(widgets::theme_border_type(theme))
                    .border_style(widgets::theme_unfocused(theme)),
            ),
        area,
    );
}

fn render_next(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.config.ui.theme;
    let mut spans = vec![
        Span::styled(" NEXT ", theme.active_tab_style()),
        Span::raw(" "),
    ];

    let mut shown = 0;
    let mut current_len = 7; // " NEXT " + " "

    let sep = if theme == Theme::Ascii { " - " } else { " · " };

    for event in &app.state.next {
        if shown >= app.config.agenda.next_events {
            break;
        }
        let event_spans = widgets::styled_relative_event_spans(app, event);
        let event_char_len: usize = event_spans.iter().map(|s| s.content.chars().count()).sum();
        let sep_len = if shown == 0 { 0 } else { sep.len() };
        let reserve = 10;

        if current_len + sep_len + event_char_len + reserve > area.width as usize {
            break;
        }

        if shown > 0 {
            spans.push(Span::styled(sep, widgets::theme_unfocused(theme)));
            current_len += sep.len();
        }
        spans.extend(event_spans);
        current_len += event_char_len;
        shown += 1;
    }

    let remaining = app.state.next_total.saturating_sub(shown);
    if remaining > 0 {
        spans.push(Span::styled(
            format!(" [+{remaining}]"),
            theme.key_badge_style(),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_status(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.config.ui.theme;
    let mode_text = match app.state.input_mode {
        InputMode::Normal => " NORMAL ",
        InputMode::Editor => " EDIT ",
        InputMode::Search => " SEARCH ",
        InputMode::LinkBank => " LINKS ",
        InputMode::LinkSearch => " LINK SEARCH ",
        InputMode::Confirm => " CONFIRM ",
        InputMode::Scope => " SCOPE ",
        InputMode::GotoDate => " DATE ",
        InputMode::CreateTask => " TASK ",
    };

    let mode_style = theme.active_tab_style();

    let mut spans = vec![
        Span::styled(mode_text, mode_style),
        Span::styled(
            format!(
                " {} › {} ",
                app.state.active_view.label(),
                app.state.selected_date.format("%d.%m.%Y")
            ),
            theme.title_style(false, false),
        ),
    ];

    let sep = if theme == Theme::Ascii { "| " } else { "│ " };
    if let Some(status) = &app.state.status_message {
        spans.push(Span::styled(sep, widgets::theme_unfocused(theme)));
        spans.push(Span::styled(status.clone(), theme.key_badge_style()));
        spans.push(Span::raw(" "));
    }

    spans.push(Span::styled(
        format!("[F5: {}] ", theme.name()),
        theme.active_tab_style(),
    ));

    let hints = [
        ("a", "ADD"),
        ("T", "TODO"),
        ("Space", "CHECK"),
        ("e", "EDIT"),
        ("d/x", "DEL"),
        ("p", "PRIORITY"),
        ("o", "PREVIEW"),
        ("y", "LINK"),
        ("n", "NEXT DAY"),
        ("Tab", "PANES"),
        ("F5", "THEME"),
        ("/", "AGENDA"),
        ("?", "HELP"),
        ("q", "QUIT"),
    ];

    let mut hints_spans = Vec::new();
    for (key, label) in hints {
        hints_spans.push(Span::styled(format!("[{key}]"), theme.key_badge_style()));
        hints_spans.push(Span::styled(
            format!(" {label} "),
            widgets::theme_unfocused(theme),
        ));
    }

    let left_len: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let hints_len: usize = hints_spans.iter().map(|s| s.content.chars().count()).sum();

    if area.width as usize > left_len + hints_len + 1 {
        let padding = (area.width as usize) - left_len - hints_len;
        spans.push(Span::raw(" ".repeat(padding)));
        spans.extend(hints_spans);
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::{DeleteTarget, Editor, EventForm, EventTarget, Popup, state::ScopeOperation},
        config::Config,
        storage::Database,
    };
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_render_all_views_and_themes_headless() {
        let db = Database::in_memory().unwrap();
        let mut app = App::new(db, Config::default()).unwrap();
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        for theme in [Theme::Default, Theme::Ascii] {
            app.config.ui.theme = theme;

            // 1. Week view
            app.state.active_view = View::Week;
            terminal.draw(|f| render(f, &app)).unwrap();

            // 2. Day view
            app.state.active_view = View::Day;
            terminal.draw(|f| render(f, &app)).unwrap();

            // 3. Month view
            app.state.active_view = View::Month;
            terminal.draw(|f| render(f, &app)).unwrap();

            // 4. Year view
            app.state.active_view = View::Year;
            terminal.draw(|f| render(f, &app)).unwrap();

            // 5. Overlays
            app.state.overlay = Some(Overlay::Agenda);
            terminal.draw(|f| render(f, &app)).unwrap();

            app.state.overlay = Some(Overlay::Upcoming);
            terminal.draw(|f| render(f, &app)).unwrap();
            app.state.overlay = None;

            // 6. Popups
            app.state.popup = Some(Popup::Help);
            terminal.draw(|f| render(f, &app)).unwrap();

            app.state.popup = Some(Popup::Confirm {
                message: "Удалить элемент?".into(),
                target: DeleteTarget::Note(1),
            });
            terminal.draw(|f| render(f, &app)).unwrap();

            let occ = crate::model::EventOccurrence {
                event_id: 1,
                date: app.state.today,
                title: "Событие".into(),
                description: None,
                directory: None,
                start_time: None,
                end_time: None,
                importance: crate::model::Importance::Normal,
                tags: Vec::new(),
                is_recurring: true,
                recurrence_id: Some(1),
                original_date: app.state.today,
                favorite_links: Vec::new(),
            };
            app.state.popup = Some(Popup::Scope(ScopeOperation::Delete(occ)));
            terminal.draw(|f| render(f, &app)).unwrap();

            app.state.popup = Some(Popup::GotoDate("2026-09-03".into()));
            terminal.draw(|f| render(f, &app)).unwrap();

            app.state.popup = Some(Popup::CreateTask("Сделать задачу".into()));
            terminal.draw(|f| render(f, &app)).unwrap();

            app.state.popup = Some(Popup::Editor(Editor::Event {
                target: EventTarget::New,
                form: EventForm::new(app.state.today),
            }));
            terminal.draw(|f| render(f, &app)).unwrap();

            app.state.popup = None;
        }
    }
}
