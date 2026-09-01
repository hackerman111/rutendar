use std::{
    error::Error,
    io::{self, Write},
};

use chrono::{Duration, Local};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal, TerminalOptions, Viewport,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    model::{Importance, NewTask, Task, TaskFilter, parse_date},
    storage::Database,
};

#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct TaskAddArgs {
    pub title: Option<String>,
    pub date: Option<String>,
    pub desc: Option<String>,
    pub importance: Option<String>,
}

impl TaskAddArgs {
    pub fn parse_from(args: &[String]) -> Self {
        let mut result = Self::default();
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--title" | "-t" => result.title = iter.next().cloned(),
                "--date" | "-d" => result.date = iter.next().cloned(),
                "--desc" => result.desc = iter.next().cloned(),
                "--importance" | "-i" => result.importance = iter.next().cloned(),
                val if !val.starts_with('-') && result.title.is_none() => {
                    result.title = Some(val.to_string());
                }
                _ => {}
            }
        }
        result
    }
}

pub fn run_task_add(database: &Database, args: &TaskAddArgs) -> Result<(), Box<dyn Error>> {
    let title = if let Some(t) = &args.title {
        t.clone()
    } else {
        print!("Название задания: ");
        io::stdout().flush()?;
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            return Err("название задания не может быть пустым".into());
        }
        trimmed
    };

    let date = if let Some(d) = &args.date {
        let clean = d.trim().to_lowercase();
        match clean.as_str() {
            "today" | "сегодня" => Some(Local::now().date_naive()),
            "tomorrow" | "завтра" => Some(Local::now().date_naive() + Duration::days(1)),
            val => Some(parse_date(val).map_err(|_| {
                format!("неверный формат даты '{val}'. Используйте DD.MM.YYYY или today/tomorrow")
            })?),
        }
    } else {
        None
    };

    let importance = match args
        .importance
        .as_deref()
        .unwrap_or("normal")
        .to_lowercase()
        .as_str()
    {
        "high" | "3" | "!" | "высокая" => Importance::High,
        "low" | "1" | "низкая" => Importance::Low,
        "none" | "0" | "нет" => Importance::None,
        _ => Importance::Normal,
    };

    let new_task = NewTask {
        title: title.clone(),
        description: args.desc.clone(),
        date,
        importance,
    };

    let id = database.create_task(&new_task)?;
    println!("\x1b[1;32m✓ Задание создано [ID: {id}]\x1b[0m");
    println!("  Название: \x1b[1m{title}\x1b[0m");
    if let Some(d) = date {
        println!("  Срок:     \x1b[36m{}\x1b[0m", d.format("%d.%m.%Y"));
    }
    if let Some(desc) = &args.desc {
        println!("  Описан.:  \x1b[2m{desc}\x1b[0m");
    }
    Ok(())
}

pub fn run_task_toggle(database: &Database, id: i64) -> Result<(), Box<dyn Error>> {
    let is_done = database.toggle_task(id)?;
    if is_done {
        println!("\x1b[1;32m✓ [x] Задание #{id} отмечено как выполненное!\x1b[0m");
    } else {
        println!("\x1b[1;33m○ [ ] Задание #{id} возвращено в список активных!\x1b[0m");
    }
    Ok(())
}

pub fn run_task_list(database: &Database, filter_str: Option<&str>) -> Result<(), Box<dyn Error>> {
    let today = Local::now().date_naive();
    let (filter, title) = match filter_str.map(|s| s.to_lowercase()).as_deref() {
        Some("today" | "сегодня") => (None, "ЗАДАНИЯ НА СЕГОДНЯ"),
        Some("done" | "выполненные") => (Some(TaskFilter::Done), "ВЫПОЛНЕННЫЕ ЗАДАНИЯ"),
        Some("all" | "все") => (Some(TaskFilter::All), "ВСЕ ЗАДАНИЯ"),
        _ => (Some(TaskFilter::Active), "АКТИВНЫЕ ЗАДАНИЯ"),
    };

    let tasks = if let Some(f) = filter {
        database.all_tasks(f)?
    } else {
        database.tasks_on_date(today)?
    };

    println!("\x1b[1;36m{title}\x1b[0m (всего: {})\n", tasks.len());
    if tasks.is_empty() {
        println!("  \x1b[2m(заданий нет)\x1b[0m");
        return Ok(());
    }

    println!(
        "  {:<5} {:<5} {:<3} {:<12} НАЗВАНИЕ",
        "ID", "СТАТ", "ВАЖ", "СРОК"
    );
    println!("  {}", "─".repeat(55));

    for task in tasks {
        let status_span = if task.is_done {
            "\x1b[32m[x]\x1b[0m"
        } else {
            "\x1b[33m[ ]\x1b[0m"
        };
        let pri_span = match task.importance {
            Importance::High => "\x1b[1;31m!\x1b[0m",
            Importance::Normal => "\x1b[38;5;214m•\x1b[0m",
            Importance::Low => "\x1b[37m·\x1b[0m",
            Importance::None => " ",
        };
        let date_span = if let Some(d) = task.date {
            if d == today {
                "\x1b[1;36mСегодня\x1b[0m".to_string()
            } else {
                d.format("%d.%m.%Y").to_string()
            }
        } else {
            "\x1b[2mБез срока\x1b[0m".to_string()
        };

        let title_fmt = if task.is_done {
            format!("\x1b[2;9m{}\x1b[0m", task.title)
        } else {
            format!("\x1b[1m{}\x1b[0m", task.title)
        };

        println!(
            "  {:<5} {}   {:<3} {:<21} {}",
            task.id, status_span, pri_span, date_span, title_fmt
        );
    }
    println!();
    Ok(())
}

pub fn run_task_menu(database: &Database) -> Result<(), Box<dyn Error>> {
    let mut filter = TaskFilter::Active;
    let mut query = String::new();
    let mut selected = 0usize;

    enable_raw_mode()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(10),
        },
    )?;

    loop {
        let all_tasks = database.all_tasks(filter)?;
        let filtered: Vec<&Task> = all_tasks
            .iter()
            .filter(|task| {
                if query.trim().is_empty() {
                    return true;
                }
                let clean = query.trim().to_lowercase();
                task.title.to_lowercase().contains(&clean)
                    || task
                        .description
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&clean)
            })
            .collect();

        if !filtered.is_empty() && selected >= filtered.len() {
            selected = filtered.len() - 1;
        }

        terminal.draw(|frame| {
            let area = frame.area();
            render_inline_task_menu(frame, area, filter, &query, &filtered, selected);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                break;
            }
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => break,
                KeyCode::Tab => {
                    filter = match filter {
                        TaskFilter::Active => TaskFilter::All,
                        TaskFilter::All => TaskFilter::Done,
                        TaskFilter::Done => TaskFilter::Active,
                    };
                    selected = 0;
                }
                KeyCode::Up | KeyCode::Char('k')
                    if !key.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    selected = selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j')
                    if !key.modifiers.contains(KeyModifiers::CONTROL) && !filtered.is_empty() =>
                {
                    selected = (selected + 1).min(filtered.len() - 1);
                }
                KeyCode::Char(' ') => {
                    if let Some(task) = filtered.get(selected) {
                        let _ = database.toggle_task(task.id);
                    }
                }
                KeyCode::Char('d') | KeyCode::Char('x') => {
                    if let Some(task) = filtered.get(selected) {
                        let _ = database.delete_task(task.id);
                        if selected > 0 && selected >= filtered.len().saturating_sub(1) {
                            selected = selected.saturating_sub(1);
                        }
                    }
                }
                KeyCode::Char('a') => {
                    // Temporarily restore terminal for line input
                    disable_raw_mode()?;
                    print!("\r\n\x1b[1;36m› Новое задание:\x1b[0m ");
                    io::stdout().flush()?;
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    let title = input.trim().to_string();
                    if !title.is_empty() {
                        let _ = database.create_task(&NewTask {
                            title,
                            description: None,
                            date: Some(Local::now().date_naive()),
                            importance: Importance::Normal,
                        });
                    }
                    enable_raw_mode()?;
                }
                KeyCode::Backspace => {
                    query.pop();
                    selected = 0;
                }
                KeyCode::Char(c) => {
                    query.push(c);
                    selected = 0;
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    terminal.clear()?;
    println!("\x1b[2mВыход из меню заданий.\x1b[0m");
    Ok(())
}

fn render_inline_task_menu(
    frame: &mut Frame,
    area: Rect,
    filter: TaskFilter,
    query: &str,
    tasks: &[&Task],
    selected: usize,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Filter tabs & Search
            Constraint::Min(3),    // Tasks list
            Constraint::Length(1), // Shortcuts footer
        ])
        .split(area);

    // Header: search & filter tabs
    let mut header_spans = vec![
        Span::styled(
            " TASKS ",
            Style::new()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ];

    let tabs = [
        (TaskFilter::Active, "Активные"),
        (TaskFilter::All, "Все"),
        (TaskFilter::Done, "Выполненные"),
    ];
    for (t_filter, label) in tabs {
        let is_active = filter == t_filter;
        if is_active {
            header_spans.push(Span::styled(
                format!(" [{label}] "),
                Style::new()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            header_spans.push(Span::styled(
                format!("  {label}  "),
                Style::new().fg(Color::DarkGray),
            ));
        }
    }

    header_spans.push(Span::styled(" │ Поиск: ", Style::new().fg(Color::DarkGray)));
    header_spans.push(Span::styled(
        if query.is_empty() { "..." } else { query },
        Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    ));

    frame.render_widget(Paragraph::new(Line::from(header_spans)), rows[0]);

    // List of tasks
    let visible_capacity = rows[1].height as usize;
    let scroll_offset = if selected >= visible_capacity {
        selected - visible_capacity + 1
    } else {
        0
    };

    let today = Local::now().date_naive();
    let mut lines = Vec::new();

    if tasks.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "   (нет заданий в этом фильтре)",
            Style::new().fg(Color::DarkGray),
        )]));
    } else {
        for (idx, task) in tasks
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_capacity)
        {
            let is_sel = idx == selected;
            let cursor = if is_sel { " › " } else { "   " };
            let cursor_style = if is_sel {
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let checkbox = if task.is_done {
                Span::styled(
                    "[x] ",
                    Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled("[ ] ", Style::new().fg(Color::Yellow))
            };

            let pri_span = match task.importance {
                Importance::High => Span::styled(
                    "! ",
                    Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Importance::Normal => Span::styled("• ", Style::new().fg(Color::Rgb(255, 165, 0))),
                Importance::Low => Span::styled("· ", Style::new().fg(Color::Gray)),
                Importance::None => Span::styled("  ", Style::default()),
            };

            let date_text = if let Some(d) = task.date {
                if d == today {
                    "Сегодня".to_string()
                } else {
                    d.format("%d.%m").to_string()
                }
            } else {
                "      ".to_string()
            };
            let date_span = Span::styled(format!("{:<8}", date_text), Style::new().fg(Color::Cyan));

            let title_style = if is_sel {
                Style::new()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if task.is_done {
                Style::new().fg(Color::DarkGray)
            } else {
                Style::new().fg(Color::White)
            };
            let title_span = Span::styled(&task.title, title_style);

            lines.push(Line::from(vec![
                Span::styled(cursor, cursor_style),
                checkbox,
                pri_span,
                date_span,
                title_span,
            ]));
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

    // Footer shortcuts
    let footer_spans = vec![
        Span::styled(
            "[Space]",
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Отметить  ", Style::new().fg(Color::DarkGray)),
        Span::styled(
            "[Tab]",
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Фильтр  ", Style::new().fg(Color::DarkGray)),
        Span::styled(
            "[a]",
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Создать  ", Style::new().fg(Color::DarkGray)),
        Span::styled(
            "[d/x]",
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Удалить  ", Style::new().fg(Color::DarkGray)),
        Span::styled(
            "[q]",
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Выход", Style::new().fg(Color::DarkGray)),
    ];
    frame.render_widget(Paragraph::new(Line::from(footer_spans)), rows[2]);
}
