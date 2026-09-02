use std::{error::Error, io, time::Duration};

use chrono::NaiveDate;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal, TerminalOptions, Viewport,
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::{
    cli::add::{parse_cli_date, parse_cli_directory, parse_cli_tags, parse_cli_time},
    completion,
    model::{EventOccurrence, Importance, NewEvent, Tag},
    storage::Database,
};

const BG_SELECTED: Color = Color::Rgb(24, 34, 52);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddFormField {
    Title,
    Date,
    Time,
    Importance,
    Tags,
    Directory,
    Description,
}

impl AddFormField {
    pub fn next(self) -> Self {
        match self {
            Self::Title => Self::Date,
            Self::Date => Self::Time,
            Self::Time => Self::Importance,
            Self::Importance => Self::Tags,
            Self::Tags => Self::Directory,
            Self::Directory => Self::Description,
            Self::Description => Self::Title,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Title => Self::Description,
            Self::Date => Self::Title,
            Self::Time => Self::Date,
            Self::Importance => Self::Time,
            Self::Tags => Self::Importance,
            Self::Directory => Self::Tags,
            Self::Description => Self::Directory,
        }
    }
}

pub struct AddFormApp {
    pub active_field: AddFormField,
    pub default_date: NaiveDate,
    pub title: String,
    pub date: String,
    pub time: String,
    pub importance: Importance,
    pub tags: String,
    pub directory: String,
    pub description: String,
    pub status_message: Option<String>,
    pub is_error: bool,
    pub suggestions: Vec<String>,
    pub suggestion_idx: usize,
}

impl AddFormApp {
    pub fn new(default_date: NaiveDate) -> Self {
        Self {
            active_field: AddFormField::Title,
            default_date,
            title: String::new(),
            date: default_date.format("%d.%m.%Y").to_string(),
            time: String::new(),
            importance: Importance::Normal,
            tags: String::new(),
            directory: String::new(),
            description: String::new(),
            status_message: None,
            is_error: false,
            suggestions: Vec::new(),
            suggestion_idx: 0,
        }
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent, db: &Database) -> bool {
        // Returns true if form should submit / save
        match (key.code, key.modifiers) {
            (KeyCode::Tab, KeyModifiers::NONE) | (KeyCode::Down, _) => {
                self.active_field = self.active_field.next();
                self.suggestions.clear();
                self.status_message = None;
            }
            (KeyCode::BackTab, _) | (KeyCode::Up, _) => {
                self.active_field = self.active_field.prev();
                self.suggestions.clear();
                self.status_message = None;
            }
            (KeyCode::Left, _) if self.active_field == AddFormField::Importance => {
                self.importance = match self.importance {
                    Importance::None => Importance::High,
                    Importance::Low => Importance::None,
                    Importance::Normal => Importance::Low,
                    Importance::High => Importance::Normal,
                };
            }
            (KeyCode::Right, _) if self.active_field == AddFormField::Importance => {
                self.importance = match self.importance {
                    Importance::None => Importance::Low,
                    Importance::Low => Importance::Normal,
                    Importance::Normal => Importance::High,
                    Importance::High => Importance::None,
                };
            }
            (KeyCode::Backspace, _) => match self.active_field {
                AddFormField::Title => {
                    self.title.pop();
                }
                AddFormField::Date => {
                    self.date.pop();
                }
                AddFormField::Time => {
                    self.time.pop();
                }
                AddFormField::Tags => {
                    self.tags.pop();
                    self.suggestions.clear();
                }
                AddFormField::Directory => {
                    self.directory.pop();
                    self.suggestions.clear();
                }
                AddFormField::Description => {
                    self.description.pop();
                }
                AddFormField::Importance => {}
            },
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                match self.active_field {
                    AddFormField::Title => self.title.push(c),
                    AddFormField::Date => self.date.push(c),
                    AddFormField::Time => self.time.push(c),
                    AddFormField::Tags => {
                        self.tags.push(c);
                        let last_word = self
                            .tags
                            .split_whitespace()
                            .next_back()
                            .unwrap_or(&self.tags);
                        self.suggestions = completion::complete_tags(db, last_word, 4);
                        self.suggestion_idx = 0;
                    }
                    AddFormField::Directory => {
                        self.directory.push(c);
                        self.suggestions = completion::complete_directories(&self.directory, 4);
                        self.suggestion_idx = 0;
                    }
                    AddFormField::Description => self.description.push(c),
                    AddFormField::Importance => match c {
                        '0' | 'n' => self.importance = Importance::None,
                        '1' | 'l' => self.importance = Importance::Low,
                        '2' => self.importance = Importance::Normal,
                        '3' | 'h' => self.importance = Importance::High,
                        _ => {}
                    },
                }
            }
            (KeyCode::Enter, _) => {
                return true;
            }
            _ => {}
        }
        false
    }

    pub fn validate_and_create(&mut self, db: &mut Database) -> Result<EventOccurrence, String> {
        let clean_title = self.title.trim().to_string();
        if clean_title.is_empty() {
            return Err("укажите название события".into());
        }

        let date = parse_cli_date(&self.date, self.default_date)
            .map_err(|e| format!("ошибка в дате: {e}"))?;
        let (start_time, end_time) =
            parse_cli_time(&self.time).map_err(|e| format!("ошибка во времени: {e}"))?;
        let directory = parse_cli_directory(&self.directory)
            .map_err(|e| format!("ошибка в директории: {e}"))?;
        let tags = parse_cli_tags(&self.tags);
        let description = if self.description.trim().is_empty() {
            None
        } else {
            Some(self.description.trim().to_string())
        };

        let new_event = NewEvent {
            title: clean_title.clone(),
            description: description.clone(),
            start_date: date,
            start_time,
            end_time,
            importance: self.importance,
            directory: directory.clone(),
        };

        let event_id = db
            .create_event(&new_event, None, &tags, &[])
            .map_err(|e| format!("ошибка базы данных: {e}"))?;

        Ok(EventOccurrence {
            event_id,
            recurrence_id: None,
            original_date: date,
            date,
            start_time,
            end_time,
            title: clean_title,
            description,
            importance: self.importance,
            tags: tags
                .into_iter()
                .map(|t| Tag {
                    id: 0,
                    name: t.clone(),
                    normalized_name: t,
                })
                .collect(),
            favorite_links: Vec::new(),
            directory,
            is_recurring: false,
        })
    }
}

pub fn render_add_form(frame: &mut Frame<'_>, area: Rect, app: &AddFormApp) {
    let title_line = Line::from(vec![
        Span::styled(
            " rutendar ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("─ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "➕ Новое событие ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "(Enter: Сохранить · Esc: Отмена) ",
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let footer_line = Line::from(vec![
        Span::styled(
            " [Tab/↓/↑] ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Поля · ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "[←/→] ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Важность · ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "[Enter] ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Сохранить · ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "[Esc] ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Отмена ", Style::default().fg(Color::DarkGray)),
    ]);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title_line)
        .title_bottom(footer_line);

    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    if inner.height < 7 {
        return;
    }

    let mut lines = vec![
        render_field_line(
            "📌 Название:    ",
            &app.title,
            app.active_field == AddFormField::Title,
            "начните вводить название события...",
        ),
        render_field_line(
            "📅 Дата:        ",
            &app.date,
            app.active_field == AddFormField::Date,
            "DD.MM.YYYY, today, tomorrow",
        ),
        render_field_line(
            "⏰ Время:       ",
            &app.time,
            app.active_field == AddFormField::Time,
            "HH:MM или HH:MM-HH:MM (пусто — весь день)",
        ),
        render_importance_line(app.importance, app.active_field == AddFormField::Importance),
        render_field_line(
            "🏷  Теги:        ",
            &app.tags,
            app.active_field == AddFormField::Tags,
            "#универ #работа (через пробел)",
        ),
        render_field_line(
            "📁 Директория:  ",
            &app.directory,
            app.active_field == AddFormField::Directory,
            "~/путь к директории проекта",
        ),
        render_field_line(
            "📝 Описание:    ",
            &app.description,
            app.active_field == AddFormField::Description,
            "дополнительные заметки и детали...",
        ),
        Line::from(Span::styled(
            " ───────────────────────────────────────────────────────────────────────────────────",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    // Status / Hint line
    if let Some(msg) = &app.status_message {
        let style = if app.is_error {
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        lines.push(Line::from(vec![
            Span::styled("  ⚠  ", style),
            Span::styled(msg, style),
        ]));
    } else if !app.suggestions.is_empty() {
        let mut spans = vec![Span::styled(
            "  💡 Варианты: ",
            Style::default().fg(Color::Yellow),
        )];
        for s in &app.suggestions {
            spans.push(Span::styled(
                format!("{s}  "),
                Style::default().fg(Color::Cyan),
            ));
        }
        lines.push(Line::from(spans));
    } else {
        lines.push(Line::from(Span::styled(
            "  ℹ  Нажмите Tab / ↓ для перехода по полям, Enter — сохранить событие",
            Style::default().fg(Color::DarkGray),
        )));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_field_line<'a>(
    label: &'a str,
    val: &'a str,
    is_active: bool,
    placeholder: &'a str,
) -> Line<'a> {
    let pointer = if is_active {
        Span::styled(
            " ▸ ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("   ")
    };

    let label_style = if is_active {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let val_span = if val.is_empty() {
        Span::styled(placeholder, Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(
            val,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
    };

    let cursor = if is_active {
        Span::styled("█", Style::default().fg(Color::Cyan))
    } else {
        Span::raw("")
    };

    let spans = vec![pointer, Span::styled(label, label_style), val_span, cursor];

    if is_active {
        Line::from(spans).style(Style::default().bg(BG_SELECTED))
    } else {
        Line::from(spans)
    }
}

fn render_importance_line(current: Importance, is_active: bool) -> Line<'static> {
    let pointer = if is_active {
        Span::styled(
            " ▸ ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("   ")
    };

    let label_style = if is_active {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let pill = |imp: Importance, title: &'static str, color: Color| {
        if current == imp {
            Span::styled(
                format!(" [ {title} ] "),
                Style::default()
                    .fg(Color::Black)
                    .bg(color)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(format!("  {title}  "), Style::default().fg(Color::DarkGray))
        }
    };

    let none_pill = pill(Importance::None, "нет", Color::DarkGray);
    let low_pill = pill(Importance::Low, "низкая", Color::Blue);
    let norm_pill = pill(Importance::Normal, "● обычная", Color::Cyan);
    let high_pill = pill(Importance::High, "▲ высокая", Color::LightRed);

    let hint = if is_active {
        Span::styled(" (←/→)", Style::default().fg(Color::DarkGray))
    } else {
        Span::raw("")
    };

    let spans = vec![
        pointer,
        Span::styled("⚡ Важность:     ", label_style),
        none_pill,
        Span::raw(" "),
        low_pill,
        Span::raw(" "),
        norm_pill,
        Span::raw(" "),
        high_pill,
        hint,
    ];

    if is_active {
        Line::from(spans).style(Style::default().bg(BG_SELECTED))
    } else {
        Line::from(spans)
    }
}

pub fn run_add_form_interactive(
    database: &mut Database,
    default_date: NaiveDate,
) -> Result<Option<EventOccurrence>, Box<dyn Error>> {
    let mut app = AddFormApp::new(default_date);

    enable_raw_mode()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(13),
        },
    )?;

    let result = loop {
        terminal.draw(|frame| {
            render_add_form(frame, frame.area(), &app);
        })?;

        if event::poll(Duration::from_millis(200))?
            && let Event::Key(key) = event::read()?
        {
            if (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
                || key.code == KeyCode::Esc
            {
                break None;
            }

            let should_submit = app.handle_key(key, database);
            if should_submit {
                match app.validate_and_create(database) {
                    Ok(occurrence) => {
                        break Some(occurrence);
                    }
                    Err(err) => {
                        app.status_message = Some(err);
                        app.is_error = true;
                    }
                }
            }
        }
    };

    terminal.clear()?;
    disable_raw_mode()?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_form_navigation_and_validation() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();
        let mut app = AddFormApp::new(today);
        let mut db = Database::in_memory().unwrap();

        assert_eq!(app.active_field, AddFormField::Title);
        assert_eq!(app.date, "03.09.2026");

        // Fails with empty title
        assert!(app.validate_and_create(&mut db).is_err());

        // Fill title
        app.title = "Тестовое событие".into();
        let created = app.validate_and_create(&mut db).unwrap();
        assert_eq!(created.title, "Тестовое событие");
        assert_eq!(created.date, today);

        // Field navigation
        app.active_field = app.active_field.next();
        assert_eq!(app.active_field, AddFormField::Date);
        app.active_field = app.active_field.prev();
        assert_eq!(app.active_field, AddFormField::Title);
    }

    #[test]
    fn test_render_add_form_headless() {
        use ratatui::backend::TestBackend;

        let backend = TestBackend::new(90, 13);
        let mut terminal = Terminal::new(backend).unwrap();

        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();
        let app = AddFormApp::new(today);

        terminal
            .draw(|frame| {
                render_add_form(frame, frame.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let rendered_text = format!("{:?}", buffer);

        assert!(rendered_text.contains("Новое событие"));
        assert!(rendered_text.contains("Название"));
        assert!(rendered_text.contains("Дата"));
        assert!(rendered_text.contains("03.09.2026"));
    }
}
