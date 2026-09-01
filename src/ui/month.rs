use chrono::{Datelike, Duration};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::widgets::{SELECTED, TODAY, TODAY_BADGE, calendar_border_style};
use crate::{
    app::App,
    calendar::{month_start, week_start},
    model::Importance,
};

pub fn render_month(frame: &mut Frame, area: Rect, app: &App) {
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
            Paragraph::new(Span::styled(
                *header,
                Style::new()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center),
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
            let day_events = app
                .state
                .occurrences
                .iter()
                .filter(|item| item.date == date);
            let has_high_importance = day_events
                .clone()
                .any(|event| event.importance == Importance::High);
            let note_count = app
                .state
                .notes
                .iter()
                .filter(|item| item.date == date)
                .count();

            let selected = date == app.state.selected_date;
            let is_today = date == app.state.today;
            let is_curr_month = date.month() == app.state.selected_date.month();

            let mut header_spans = Vec::new();
            if selected {
                header_spans.push(Span::styled(format!(" {:02} ", date.day()), SELECTED));
            } else if is_today {
                header_spans.push(Span::styled(format!(" {:02} ", date.day()), TODAY_BADGE));
            } else if is_curr_month {
                header_spans.push(Span::styled(
                    format!(" {:02} ", date.day()),
                    Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
                ));
            } else {
                header_spans.push(Span::styled(
                    format!(" {:02} ", date.day()),
                    Style::new().fg(Color::DarkGray),
                ));
            }

            if is_today && !selected {
                header_spans.push(Span::styled(" •", TODAY));
            }

            let mut lines = vec![Line::from(header_spans)];

            let mut metric_spans = Vec::new();
            for event in day_events {
                metric_spans.push(month_event_marker(
                    event.importance,
                    event.is_recurring,
                    is_curr_month,
                ));
            }
            if note_count > 0 {
                metric_spans.push(Span::styled(
                    format!(" ◆{note_count}"),
                    if is_curr_month {
                        Style::new().fg(Color::Yellow)
                    } else {
                        Style::new().fg(Color::DarkGray)
                    },
                ));
            }

            if !metric_spans.is_empty() {
                lines.push(Line::from(metric_spans));
            }

            let border_style = calendar_border_style(selected, is_today, has_high_importance);

            frame.render_widget(
                Paragraph::new(lines).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border_style),
                ),
                columns[day],
            );
        }
    }
}

fn month_event_marker(
    importance: Importance,
    recurring: bool,
    is_current_month: bool,
) -> Span<'static> {
    let symbol = if recurring { "↻" } else { "●" };
    let marker = if importance == Importance::High {
        format!(" !{symbol}")
    } else {
        format!(" {symbol}")
    };
    let style = if !is_current_month {
        Style::new().fg(Color::DarkGray)
    } else {
        match importance {
            Importance::High => Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
            Importance::Normal => Style::new().fg(Color::Rgb(255, 165, 0)),
            Importance::Low => Style::new().fg(Color::Gray),
            Importance::None => Style::new().fg(Color::DarkGray),
        }
    };
    Span::styled(marker, style)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_markers_distinguish_priority_and_recurrence() {
        let low = month_event_marker(Importance::Low, false, true);
        assert_eq!(low.content, " ●");
        assert_eq!(low.style, Style::new().fg(Color::Gray));

        let normal = month_event_marker(Importance::Normal, true, true);
        assert_eq!(normal.content, " ↻");
        assert_eq!(normal.style, Style::new().fg(Color::Rgb(255, 165, 0)));

        let high = month_event_marker(Importance::High, false, true);
        assert_eq!(high.content, " !●");
        assert_eq!(
            high.style,
            Style::new().fg(Color::Red).add_modifier(Modifier::BOLD)
        );
    }
}
