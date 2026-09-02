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
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    cli::add::{parse_cli_date, parse_cli_directory, parse_cli_tags, parse_cli_time},
    completion,
    model::{EventOccurrence, Importance, NewEvent, NewTask, Tag},
    storage::Database,
    ui::Theme,
};

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
    pub is_edit_mode: bool,
    pub theme: Theme,
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
            is_edit_mode: false,
            theme: Theme::default(),
        }
    }

    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn from_occurrence(occurrence: &EventOccurrence) -> Self {
        let time_str = match (occurrence.start_time, occurrence.end_time) {
            (Some(s), Some(e)) => format!("{}-{}", s.format("%H:%M"), e.format("%H:%M")),
            (Some(s), None) => s.format("%H:%M").to_string(),
            _ => String::new(),
        };

        let tags_str = occurrence
            .tags
            .iter()
            .map(|t| format!("#{}", t.name))
            .collect::<Vec<_>>()
            .join(" ");

        Self {
            active_field: AddFormField::Title,
            default_date: occurrence.date,
            title: occurrence.title.clone(),
            date: occurrence.date.format("%d.%m.%Y").to_string(),
            time: time_str,
            importance: occurrence.importance,
            tags: tags_str,
            directory: occurrence
                .directory
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
            description: occurrence.description.as_deref().unwrap_or("").to_string(),
            status_message: None,
            is_error: false,
            suggestions: Vec::new(),
            suggestion_idx: 0,
            is_edit_mode: true,
            theme: Theme::default(),
        }
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent, db: &Database) -> bool {
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

    pub fn save_event_update(
        &mut self,
        db: &mut Database,
        occurrence: &EventOccurrence,
    ) -> Result<(), String> {
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
            title: clean_title,
            description,
            start_date: date,
            start_time,
            end_time,
            importance: self.importance,
            directory,
        };

        let recurrence = db
            .get_event(occurrence.event_id)
            .ok()
            .flatten()
            .and_then(|e| e.recurrence_id)
            .and_then(|r_id| db.get_recurrence(r_id).ok().flatten())
            .map(|r| crate::model::NewRecurrence {
                interval: r.interval,
                weekdays: r.weekdays,
                start_date: date,
                end_date: r.end_date,
                count: r.count,
            });

        let favorite_link_ids: Vec<i64> = occurrence.favorite_links.iter().map(|f| f.id).collect();

        db.update_event(
            occurrence.event_id,
            &new_event,
            recurrence.as_ref(),
            &tags,
            &favorite_link_ids,
        )
        .map_err(|e| format!("ошибка базы данных: {e}"))?;

        Ok(())
    }
}

pub fn render_add_form(frame: &mut Frame<'_>, area: Rect, app: &AddFormApp) {
    let mode_title = if app.is_edit_mode {
        format!("{}Редактирование события ", app.theme.edit_icon())
    } else {
        format!("{}Новое событие ", app.theme.add_icon())
    };

    let sep = if app.theme == Theme::Plain {
        "- "
    } else {
        "─ "
    };

    let title_line = Line::from(vec![
        Span::styled(" rutendar ", app.theme.key_badge_style()),
        Span::styled(sep, app.theme.inactive_tab_style()),
        Span::styled(mode_title, app.theme.title_style(true, false)),
        Span::styled(
            "(Enter: Сохранить · Esc: Отмена) ",
            app.theme.inactive_tab_style(),
        ),
    ]);

    let footer_line = Line::from(vec![
        Span::styled(" [Tab/↓/↑] ", app.theme.key_badge_style()),
        Span::styled("Поля · ", app.theme.inactive_tab_style()),
        Span::styled("[←/→] ", app.theme.key_badge_style()),
        Span::styled("Важность · ", app.theme.inactive_tab_style()),
        Span::styled("[Enter] ", app.theme.key_badge_style()),
        Span::styled("Сохранить · ", app.theme.inactive_tab_style()),
        Span::styled("[Esc] ", app.theme.key_badge_style()),
        Span::styled("Отмена ", app.theme.inactive_tab_style()),
    ]);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(app.theme.border_type())
        .border_style(Style::default().fg(app.theme.border_color(app.is_error)))
        .title(title_line)
        .title_bottom(footer_line);

    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    if inner.height < 7 {
        return;
    }

    let divider = if app.theme == Theme::Plain {
        " -----------------------------------------------------------------------------------"
    } else {
        " ───────────────────────────────────────────────────────────────────────────────────"
    };

    let mut lines = vec![
        render_field_line(
            "📌 Название:    ",
            &app.title,
            app.active_field == AddFormField::Title,
            "начните вводить название события...",
            app.theme,
        ),
        render_field_line(
            "📅 Дата:        ",
            &app.date,
            app.active_field == AddFormField::Date,
            "DD.MM.YYYY, today, tomorrow",
            app.theme,
        ),
        render_field_line(
            "⏰ Время:       ",
            &app.time,
            app.active_field == AddFormField::Time,
            "HH:MM или HH:MM-HH:MM (пусто — весь день)",
            app.theme,
        ),
        render_importance_line(
            app.importance,
            app.active_field == AddFormField::Importance,
            app.theme,
        ),
        render_field_line(
            "🏷  Теги:        ",
            &app.tags,
            app.active_field == AddFormField::Tags,
            "#универ #работа (через пробел)",
            app.theme,
        ),
        render_field_line(
            "📁 Директория:  ",
            &app.directory,
            app.active_field == AddFormField::Directory,
            "~/путь к директории проекта",
            app.theme,
        ),
        render_field_line(
            "📝 Описание:    ",
            &app.description,
            app.active_field == AddFormField::Description,
            "дополнительные заметки и детали...",
            app.theme,
        ),
        Line::from(Span::styled(divider, app.theme.inactive_tab_style())),
    ];

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
        let mut spans = vec![Span::styled("  💡 Варианты: ", app.theme.key_badge_style())];
        for s in &app.suggestions {
            spans.push(Span::styled(format!("{s}  "), app.theme.tag_style()));
        }
        lines.push(Line::from(spans));
    } else {
        lines.push(Line::from(Span::styled(
            "  ℹ  Нажмите Tab / ↓ для перехода по полям, Enter — сохранить событие",
            app.theme.inactive_tab_style(),
        )));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_field_line<'a>(
    label: &'a str,
    val: &'a str,
    is_active: bool,
    placeholder: &'a str,
    theme: Theme,
) -> Line<'a> {
    let pointer = theme.cursor_marker(is_active);
    let label_style = if is_active {
        theme.key_badge_style()
    } else {
        theme.title_style(false, false)
    };

    let val_span = if val.is_empty() {
        Span::styled(placeholder, theme.inactive_tab_style())
    } else {
        Span::styled(val, theme.title_style(is_active, false))
    };

    let cursor = if is_active {
        if theme == Theme::Plain {
            Span::styled("_", theme.key_badge_style())
        } else {
            Span::styled("█", theme.key_badge_style())
        }
    } else {
        Span::raw("")
    };

    let spans = vec![pointer, Span::styled(label, label_style), val_span, cursor];

    if is_active {
        Line::from(spans).style(theme.selection_style())
    } else {
        Line::from(spans)
    }
}

fn render_importance_line(current: Importance, is_active: bool, theme: Theme) -> Line<'static> {
    let pointer = theme.cursor_marker(is_active);
    let label_style = if is_active {
        theme.key_badge_style()
    } else {
        theme.title_style(false, false)
    };

    let pill = |imp: Importance, title: &'static str| {
        if current == imp {
            Span::styled(format!(" [ {title} ] "), theme.active_tab_style())
        } else {
            Span::styled(format!("  {title}  "), theme.inactive_tab_style())
        }
    };

    let none_pill = pill(Importance::None, "нет");
    let low_pill = pill(Importance::Low, "низкая");
    let norm_pill = pill(Importance::Normal, "обычная");
    let high_pill = pill(Importance::High, "высокая");

    let hint = if is_active {
        Span::styled(" (←/→)", theme.inactive_tab_style())
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
        Line::from(spans).style(theme.selection_style())
    } else {
        Line::from(spans)
    }
}

pub fn run_add_form_interactive(
    database: &mut Database,
    default_date: NaiveDate,
    theme: Theme,
) -> Result<Option<EventOccurrence>, Box<dyn Error>> {
    let mut app = AddFormApp::new(default_date).with_theme(theme);

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

pub fn run_edit_form_interactive(
    database: &mut Database,
    occurrence: &EventOccurrence,
    theme: Theme,
) -> Result<bool, Box<dyn Error>> {
    let mut app = AddFormApp::from_occurrence(occurrence).with_theme(theme);

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
                break false;
            }

            let should_submit = app.handle_key(key, database);
            if should_submit {
                match app.save_event_update(database, occurrence) {
                    Ok(()) => {
                        break true;
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

// ─────────────────────────────────────────────────────────────────────────────
// Task Form Implementation
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskFormField {
    Title,
    Date,
    Importance,
    Description,
}

impl TaskFormField {
    pub fn next(self) -> Self {
        match self {
            Self::Title => Self::Date,
            Self::Date => Self::Importance,
            Self::Importance => Self::Description,
            Self::Description => Self::Title,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Title => Self::Description,
            Self::Date => Self::Title,
            Self::Importance => Self::Date,
            Self::Description => Self::Importance,
        }
    }
}

pub struct TaskFormApp {
    pub active_field: TaskFormField,
    pub default_date: NaiveDate,
    pub title: String,
    pub date: String,
    pub importance: Importance,
    pub description: String,
    pub status_message: Option<String>,
    pub is_error: bool,
    pub theme: Theme,
}

impl TaskFormApp {
    pub fn new(default_date: NaiveDate) -> Self {
        Self {
            active_field: TaskFormField::Title,
            default_date,
            title: String::new(),
            date: default_date.format("%d.%m.%Y").to_string(),
            importance: Importance::Normal,
            description: String::new(),
            status_message: None,
            is_error: false,
            theme: Theme::default(),
        }
    }

    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        match (key.code, key.modifiers) {
            (KeyCode::Tab, KeyModifiers::NONE) | (KeyCode::Down, _) => {
                self.active_field = self.active_field.next();
                self.status_message = None;
            }
            (KeyCode::BackTab, _) | (KeyCode::Up, _) => {
                self.active_field = self.active_field.prev();
                self.status_message = None;
            }
            (KeyCode::Left, _) if self.active_field == TaskFormField::Importance => {
                self.importance = match self.importance {
                    Importance::None => Importance::High,
                    Importance::Low => Importance::None,
                    Importance::Normal => Importance::Low,
                    Importance::High => Importance::Normal,
                };
            }
            (KeyCode::Right, _) if self.active_field == TaskFormField::Importance => {
                self.importance = match self.importance {
                    Importance::None => Importance::Low,
                    Importance::Low => Importance::Normal,
                    Importance::Normal => Importance::High,
                    Importance::High => Importance::None,
                };
            }
            (KeyCode::Backspace, _) => match self.active_field {
                TaskFormField::Title => {
                    self.title.pop();
                }
                TaskFormField::Date => {
                    self.date.pop();
                }
                TaskFormField::Description => {
                    self.description.pop();
                }
                TaskFormField::Importance => {}
            },
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                match self.active_field {
                    TaskFormField::Title => self.title.push(c),
                    TaskFormField::Date => self.date.push(c),
                    TaskFormField::Description => self.description.push(c),
                    TaskFormField::Importance => match c {
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

    pub fn validate_and_create(&mut self, db: &Database) -> Result<i64, String> {
        let clean_title = self.title.trim().to_string();
        if clean_title.is_empty() {
            return Err("укажите название задачи".into());
        }

        let date = if self.date.trim().is_empty() {
            None
        } else {
            Some(
                parse_cli_date(&self.date, self.default_date)
                    .map_err(|e| format!("ошибка в дате: {e}"))?,
            )
        };

        let description = if self.description.trim().is_empty() {
            None
        } else {
            Some(self.description.trim().to_string())
        };

        let new_task = NewTask {
            title: clean_title,
            description,
            date,
            importance: self.importance,
        };

        db.create_task(&new_task)
            .map_err(|e| format!("ошибка базы данных: {e}"))
    }
}

pub fn render_task_form(frame: &mut Frame<'_>, area: Rect, app: &TaskFormApp) {
    let mode_title = format!("{}Новая задача (TODO) ", app.theme.task_icon());
    let sep = if app.theme == Theme::Plain {
        "- "
    } else {
        "─ "
    };

    let title_line = Line::from(vec![
        Span::styled(" rutendar ", app.theme.key_badge_style()),
        Span::styled(sep, app.theme.inactive_tab_style()),
        Span::styled(mode_title, app.theme.title_style(true, false)),
        Span::styled(
            "(Enter: Сохранить · Esc: Отмена) ",
            app.theme.inactive_tab_style(),
        ),
    ]);

    let footer_line = Line::from(vec![
        Span::styled(" [Tab/↓/↑] ", app.theme.key_badge_style()),
        Span::styled("Поля · ", app.theme.inactive_tab_style()),
        Span::styled("[←/→] ", app.theme.key_badge_style()),
        Span::styled("Важность · ", app.theme.inactive_tab_style()),
        Span::styled("[Enter] ", app.theme.key_badge_style()),
        Span::styled("Сохранить · ", app.theme.inactive_tab_style()),
        Span::styled("[Esc] ", app.theme.key_badge_style()),
        Span::styled("Отмена ", app.theme.inactive_tab_style()),
    ]);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(app.theme.border_type())
        .border_style(Style::default().fg(app.theme.border_color(app.is_error)))
        .title(title_line)
        .title_bottom(footer_line);

    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    if inner.height < 6 {
        return;
    }

    let divider = if app.theme == Theme::Plain {
        " -----------------------------------------------------------------------------------"
    } else {
        " ───────────────────────────────────────────────────────────────────────────────────"
    };

    let mut lines = vec![
        render_field_line(
            "📌 Название задачи: ",
            &app.title,
            app.active_field == TaskFormField::Title,
            "кратко суть задачи...",
            app.theme,
        ),
        render_field_line(
            "📅 Срок (дата):     ",
            &app.date,
            app.active_field == TaskFormField::Date,
            "DD.MM.YYYY, today, tomorrow (или пусто)",
            app.theme,
        ),
        render_importance_line(
            app.importance,
            app.active_field == TaskFormField::Importance,
            app.theme,
        ),
        render_field_line(
            "📝 Описание:        ",
            &app.description,
            app.active_field == TaskFormField::Description,
            "дополнительные заметки к задаче...",
            app.theme,
        ),
        Line::from(Span::styled(divider, app.theme.inactive_tab_style())),
    ];

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
    } else {
        lines.push(Line::from(Span::styled(
            "  ℹ  Нажмите Tab / ↓ для перехода по полям, Enter — сохранить задачу",
            app.theme.inactive_tab_style(),
        )));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

pub fn run_add_task_interactive(
    database: &mut Database,
    default_date: NaiveDate,
    theme: Theme,
) -> Result<bool, Box<dyn Error>> {
    let mut app = TaskFormApp::new(default_date).with_theme(theme);

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
            render_task_form(frame, frame.area(), &app);
        })?;

        if event::poll(Duration::from_millis(200))?
            && let Event::Key(key) = event::read()?
        {
            if (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
                || key.code == KeyCode::Esc
            {
                break false;
            }

            let should_submit = app.handle_key(key);
            if should_submit {
                match app.validate_and_create(database) {
                    Ok(_) => {
                        break true;
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

        // Test from_occurrence and edit update
        let mut edit_app = AddFormApp::from_occurrence(&created);
        assert!(edit_app.is_edit_mode);
        assert_eq!(edit_app.title, "Тестовое событие");
        edit_app.title = "Обновленное событие".into();
        assert!(edit_app.save_event_update(&mut db, &created).is_ok());

        let loaded = db.get_event(created.event_id).unwrap().unwrap();
        assert_eq!(loaded.title, "Обновленное событие");
    }

    #[test]
    fn test_task_form_navigation_and_creation() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();
        let mut app = TaskFormApp::new(today);
        let db = Database::in_memory().unwrap();

        assert_eq!(app.active_field, TaskFormField::Title);
        assert!(app.validate_and_create(&db).is_err());

        app.title = "Купить молоко".into();
        let task_id = app.validate_and_create(&db).unwrap();
        assert!(task_id > 0);

        let tasks = db.tasks_on_date(today).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Купить молоко");
    }

    #[test]
    fn test_render_add_and_task_forms_headless() {
        use ratatui::backend::TestBackend;

        let backend = TestBackend::new(90, 13);
        let mut terminal = Terminal::new(backend).unwrap();

        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();
        let app = AddFormApp::new(today).with_theme(Theme::Plain);
        terminal
            .draw(|frame| {
                render_add_form(frame, frame.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let rendered_text = format!("{:?}", buffer);
        assert!(rendered_text.contains("Новое событие"));

        let task_app = TaskFormApp::new(today).with_theme(Theme::Plain);
        terminal
            .draw(|frame| {
                render_task_form(frame, frame.area(), &task_app);
            })
            .unwrap();

        let task_buffer = terminal.backend().buffer();
        let task_text = format!("{:?}", task_buffer);
        assert!(task_text.contains("Новая задача"));
    }
}
