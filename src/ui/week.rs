use chrono::{Datelike, Duration};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::{
    day::render_day,
    widgets::{
        SELECTED, calendar_border_style, styled_event_spans, styled_tags_line, weekday_short,
    },
};
use crate::{app::App, calendar::week_start, model::Importance};

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
        let tasks: Vec<_> = app
            .state
            .tasks
            .iter()
            .filter(|task| task.date == Some(date))
            .collect();
        let selected = date == app.state.selected_date;
        let is_today = date == app.state.today;
        let has_high_importance = events
            .iter()
            .any(|event| event.importance == Importance::High)
            || tasks.iter().any(|task| task.importance == Importance::High);

        let title_spans = if is_today {
            vec![
                Span::styled(
                    format!(" {} {:02} ", weekday_short(date), date.day()),
                    Style::new()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " TODAY",
                    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
            ]
        } else if selected {
            vec![Span::styled(
                format!(" {} {:02} ", weekday_short(date), date.day()),
                SELECTED,
            )]
        } else {
            vec![Span::styled(
                format!("  {} {:02}  ", weekday_short(date), date.day()),
                Style::new().fg(Color::DarkGray),
            )]
        };

        let show_tags = column.width > 15
            && events.len().saturating_mul(2) <= column.height.saturating_sub(2) as usize;
        let lines_per_event = if show_tags { 2 } else { 1 };
        let capacity = (column.height.saturating_sub(2) as usize / lines_per_event).max(1);

        let mut item_lines = Vec::new();
        for (index, event) in events.iter().enumerate() {
            let is_event_selected = selected && index == app.state.selected_event;
            item_lines.push(Line::from(styled_event_spans(
                app,
                event,
                is_event_selected,
            )));
            if show_tags && !event.tags.is_empty() {
                item_lines.push(styled_tags_line(event, is_event_selected));
            }
        }
        for (k, task) in tasks.iter().enumerate() {
            let item_index = events.len() + k;
            let is_task_selected = selected && item_index == app.state.selected_event;
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
            let title_style = if is_task_selected {
                SELECTED
            } else if task.is_done {
                Style::new().fg(Color::DarkGray)
            } else {
                Style::new().fg(Color::White)
            };
            let title_span = Span::styled(&task.title, title_style);
            item_lines.push(Line::from(vec![checkbox, imp_span, title_span]));
        }

        let start_idx = if selected {
            app.state
                .selected_event
                .saturating_sub(capacity.saturating_sub(1))
        } else {
            0
        };

        let lines = item_lines
            .into_iter()
            .skip(start_idx)
            .take(capacity)
            .collect::<Vec<_>>();

        let border_style = calendar_border_style(selected, is_today, has_high_importance);

        frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Line::from(title_spans))
                    .border_style(border_style),
            ),
            *column,
        );
    }
}
