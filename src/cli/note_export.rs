use std::{
    collections::HashSet,
    error::Error,
    io::{self, stdout},
    path::PathBuf,
};

use chrono::{Datelike, Local, NaiveDate};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    calendar::{month_end, month_start, week_end, week_start},
    model::{Note, NoteId},
    storage::Database,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotesPeriod {
    Day,
    Week,
    Month,
    All,
}

impl NotesPeriod {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Day => "День",
            Self::Week => "Неделя",
            Self::Month => "Месяц",
            Self::All => "Все",
        }
    }

    pub fn file_suffix(&self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
            Self::All => "all",
        }
    }

    pub fn date_range(&self, today: NaiveDate) -> Option<(NaiveDate, NaiveDate)> {
        match self {
            Self::Day => Some((today, today)),
            Self::Week => Some((week_start(today), week_end(today))),
            Self::Month => Some((month_start(today), month_end(today))),
            Self::All => None,
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Day => Self::Week,
            Self::Week => Self::Month,
            Self::Month => Self::All,
            Self::All => Self::Day,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Self::Day => Self::All,
            Self::Week => Self::Day,
            Self::Month => Self::Week,
            Self::All => Self::Month,
        }
    }
}

pub fn format_notes_markdown(notes: &[Note], period_label: &str) -> String {
    let mut out = String::new();
    out.push_str("# Экспорт заметок Rutendar\n");
    out.push_str(&format!(
        "_Период: {} | Всего заметок: {}_\n\n",
        period_label,
        notes.len()
    ));

    if notes.is_empty() {
        out.push_str("*(заметок за данный период нет)*\n");
        return out;
    }

    for (i, note) in notes.iter().enumerate() {
        if i > 0 {
            out.push_str("\n---\n\n");
        }
        let date_str = note.date.format("%d.%m.%Y").to_string();
        if let Some(title) = &note.title {
            out.push_str(&format!("## {} — {}\n\n", date_str, title.trim()));
        } else {
            out.push_str(&format!("## {}\n\n", date_str));
        }

        out.push_str(note.body.trim());
        out.push('\n');

        if !note.links.is_empty() {
            out.push_str("\n### Ссылки:\n");
            for link in &note.links {
                out.push_str(&format!("- [{}]({})\n", link.label.trim(), link.url.trim()));
            }
        }
    }

    out
}

pub fn fetch_notes_for_period(
    database: &Database,
    period: NotesPeriod,
    today: NaiveDate,
) -> Result<Vec<Note>, Box<dyn Error>> {
    if let Some((start, end)) = period.date_range(today) {
        Ok(database.notes_between(start, end)?)
    } else {
        Ok(database.all_notes()?)
    }
}

pub fn run_notes_export(
    database: &Database,
    period: NotesPeriod,
    target_date: Option<NaiveDate>,
    file_path: Option<&str>,
    stdout_mode: bool,
) -> Result<(), Box<dyn Error>> {
    let today = target_date.unwrap_or_else(|| Local::now().date_naive());
    let notes = fetch_notes_for_period(database, period, today)?;
    let period_desc = match period {
        NotesPeriod::Day => format!("День ({})", today.format("%d.%m.%Y")),
        NotesPeriod::Week => {
            let (s, e) = (week_start(today), week_end(today));
            format!(
                "Неделя ({} — {})",
                s.format("%d.%m.%Y"),
                e.format("%d.%m.%Y")
            )
        }
        NotesPeriod::Month => {
            format!("Месяц ({:02}.{})", today.month(), today.year())
        }
        NotesPeriod::All => "Все заметки".to_string(),
    };

    let content = format_notes_markdown(&notes, &period_desc);

    if stdout_mode {
        print!("{content}");
        return Ok(());
    }

    let target_path = if let Some(path) = file_path {
        PathBuf::from(path)
    } else {
        PathBuf::from(format!(
            "notes_{}_{}.md",
            period.file_suffix(),
            today.format("%Y%m%d")
        ))
    };

    std::fs::write(&target_path, content)?;
    println!(
        "\x1b[32m✓\x1b[0m Экспортировано заметок: \x1b[1m{}\x1b[0m в файл: \x1b[36m{}\x1b[0m",
        notes.len(),
        target_path.display()
    );

    Ok(())
}

pub fn run_notes_menu(database: &Database) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    crossterm::execute!(stdout, crossterm::cursor::Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::with_options(
        backend,
        ratatui::TerminalOptions {
            viewport: ratatui::Viewport::Inline(10),
        },
    )?;

    let res = notes_menu_loop(&mut terminal, database);

    disable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::cursor::Show)?;
    terminal.clear()?;

    if let Ok(Some(msg)) = res {
        println!("{msg}");
    } else if let Err(e) = res {
        eprintln!("\x1b[31mОшибка: {e}\x1b[0m");
    }

    Ok(())
}

fn notes_menu_loop(
    terminal: &mut ratatui::Terminal<CrosstermBackend<io::Stdout>>,
    database: &Database,
) -> Result<Option<String>, Box<dyn Error>> {
    let today = Local::now().date_naive();
    let mut period = NotesPeriod::All;
    let mut selected_index = 0usize;
    let mut chosen_ids: HashSet<NoteId> = HashSet::new();
    let mut query = String::new();
    let mut status_msg: Option<String> = None;

    loop {
        let all_period_notes = fetch_notes_for_period(database, period, today)?;
        let filtered: Vec<Note> = if query.is_empty() {
            all_period_notes
        } else {
            let q = query.to_lowercase();
            all_period_notes
                .into_iter()
                .filter(|n| {
                    n.title.as_deref().unwrap_or("").to_lowercase().contains(&q)
                        || n.body.to_lowercase().contains(&q)
                })
                .collect()
        };

        if selected_index >= filtered.len() {
            selected_index = filtered.len().saturating_sub(1);
        }

        terminal.draw(|frame| {
            render_notes_menu(
                frame,
                frame.area(),
                period,
                &filtered,
                selected_index,
                &chosen_ids,
                &query,
                status_msg.as_deref(),
            );
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(None);
                }
                KeyCode::Esc => return Ok(None),
                KeyCode::Char('q') if query.is_empty() => return Ok(None),
                KeyCode::Tab => {
                    period = period.next();
                    selected_index = 0;
                }
                KeyCode::BackTab => {
                    period = period.prev();
                    selected_index = 0;
                }
                KeyCode::Up | KeyCode::Char('k')
                    if !key.modifiers.contains(KeyModifiers::CONTROL) && query.is_empty() =>
                {
                    selected_index = selected_index.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j')
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && query.is_empty()
                        && !filtered.is_empty() =>
                {
                    selected_index = (selected_index + 1).min(filtered.len() - 1);
                }
                KeyCode::Char(' ') => {
                    if let Some(note) = filtered.get(selected_index) {
                        if chosen_ids.contains(&note.id) {
                            chosen_ids.remove(&note.id);
                        } else {
                            chosen_ids.insert(note.id);
                        }
                    }
                }
                KeyCode::Char('a') if query.is_empty() => {
                    let all_in_filter_selected =
                        !filtered.is_empty() && filtered.iter().all(|n| chosen_ids.contains(&n.id));
                    if all_in_filter_selected {
                        for n in &filtered {
                            chosen_ids.remove(&n.id);
                        }
                    } else {
                        for n in &filtered {
                            chosen_ids.insert(n.id);
                        }
                    }
                }
                KeyCode::Char('e') if query.is_empty() => {
                    let to_export: Vec<Note> = if chosen_ids.is_empty() {
                        filtered.clone()
                    } else {
                        let all_notes = database.all_notes()?;
                        all_notes
                            .into_iter()
                            .filter(|n| chosen_ids.contains(&n.id))
                            .collect()
                    };

                    if to_export.is_empty() {
                        status_msg = Some("Нет заметок для экспорта".into());
                        continue;
                    }

                    let filename = format!(
                        "notes_export_{}_{}.md",
                        period.file_suffix(),
                        today.format("%Y%m%d")
                    );
                    let label = if chosen_ids.is_empty() {
                        period.label().to_string()
                    } else {
                        format!("Выбранные ({})", to_export.len())
                    };
                    let content = format_notes_markdown(&to_export, &label);
                    std::fs::write(&filename, content)?;
                    return Ok(Some(format!(
                        "\x1b[32m✓\x1b[0m Экспортировано заметок: \x1b[1m{}\x1b[0m в файл \x1b[36m{}\x1b[0m",
                        to_export.len(),
                        filename
                    )));
                }
                KeyCode::Backspace => {
                    query.pop();
                    selected_index = 0;
                }
                KeyCode::Char(c) => {
                    query.push(c);
                    selected_index = 0;
                }
                _ => {}
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_notes_menu(
    frame: &mut Frame,
    area: Rect,
    period: NotesPeriod,
    notes: &[Note],
    selected: usize,
    chosen: &HashSet<NoteId>,
    query: &str,
    status: Option<&str>,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header + tabs
            Constraint::Min(4),    // List of notes
            Constraint::Length(1), // Shortcuts / status
        ])
        .split(area);

    // Header tabs
    let periods = [
        NotesPeriod::Day,
        NotesPeriod::Week,
        NotesPeriod::Month,
        NotesPeriod::All,
    ];
    let mut tab_spans = vec![Span::styled(
        " 📝 ЗАМЕТКИ ",
        Style::new()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )];

    for p in periods {
        let is_active = p == period;
        let style = if is_active {
            Style::new()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(Color::DarkGray)
        };
        tab_spans.push(Span::styled(format!(" [ {} ]", p.label()), style));
    }

    if !query.is_empty() {
        tab_spans.push(Span::styled(
            format!("  Поиск: \"{}\"", query),
            Style::new().fg(Color::Yellow),
        ));
    }

    let chosen_count = chosen.len();
    if chosen_count > 0 {
        tab_spans.push(Span::styled(
            format!("  Выбрано: {}", chosen_count),
            Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(tab_spans)), rows[0]);

    // List of notes
    let visible_capacity = rows[1].height as usize;
    let scroll_offset = if selected >= visible_capacity {
        selected - visible_capacity + 1
    } else {
        0
    };

    let mut lines = Vec::new();
    if notes.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "   (нет заметок в этом периоде)",
            Style::new().fg(Color::DarkGray),
        )]));
    } else {
        for (idx, note) in notes
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_capacity)
        {
            let is_sel = idx == selected;
            let cursor = if is_sel { " › " } else { "   " };
            let cursor_style = if is_sel {
                Style::new()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let is_marked = chosen.contains(&note.id);
            let check_span = if is_marked {
                Span::styled(
                    "[*] ",
                    Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled("[ ] ", Style::new().fg(Color::DarkGray))
            };

            let date_span = Span::styled(
                format!("{} ", note.date.format("%d.%m")),
                Style::new().fg(Color::Cyan),
            );

            let title_text = if let Some(t) = &note.title {
                t.as_str()
            } else {
                note.body.lines().next().unwrap_or("(пусто)")
            };

            let title_style = if is_sel {
                cursor_style
            } else {
                Style::new().fg(Color::White)
            };
            let title_span = Span::styled(title_text, title_style);

            let mut row_spans = vec![
                Span::styled(cursor, cursor_style),
                check_span,
                date_span,
                title_span,
            ];

            if !note.links.is_empty() {
                row_spans.push(Span::styled(
                    format!(" [🔗 {}]", note.links.len()),
                    Style::new().fg(Color::Yellow),
                ));
            }

            lines.push(Line::from(row_spans));
        }
    }

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::new().fg(Color::DarkGray)),
        ),
        rows[1],
    );

    // Footer
    if let Some(msg) = status {
        frame.render_widget(
            Paragraph::new(Span::styled(
                msg,
                Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            rows[2],
        );
    } else {
        let footer_spans = vec![
            Span::styled(
                "[Space]",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Выбрать  ", Style::new().fg(Color::DarkGray)),
            Span::styled(
                "[Tab]",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Период  ", Style::new().fg(Color::DarkGray)),
            Span::styled(
                "[a]",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Все  ", Style::new().fg(Color::DarkGray)),
            Span::styled(
                "[e]",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Экспорт в MD  ", Style::new().fg(Color::DarkGray)),
            Span::styled(
                "[q/Esc]",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Выход", Style::new().fg(Color::DarkGray)),
        ];
        frame.render_widget(Paragraph::new(Line::from(footer_spans)), rows[2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Link;

    #[test]
    fn format_notes_markdown_includes_all_fields_and_links() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let notes = vec![
            Note {
                id: 1,
                date,
                title: Some("Покупки".into()),
                body: "Купить молоко и хлеб".into(),
                links: vec![Link {
                    id: 10,
                    note_id: 1,
                    label: "Магазин".into(),
                    url: "https://shop.com".into(),
                }],
            },
            Note {
                id: 2,
                date,
                title: None,
                body: "Мысль дня".into(),
                links: Vec::new(),
            },
        ];

        let md = format_notes_markdown(&notes, "Неделя");
        assert!(md.contains("# Экспорт заметок Rutendar"));
        assert!(md.contains("_Период: Неделя | Всего заметок: 2_"));
        assert!(md.contains("## 01.09.2026 — Покупки"));
        assert!(md.contains("Купить молоко и хлеб"));
        assert!(md.contains("- [Магазин](https://shop.com)"));
        assert!(md.contains("## 01.09.2026\n\nМысль дня"));
    }
}
