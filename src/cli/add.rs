use std::{
    error::Error,
    io::{self, Write},
    path::PathBuf,
};

use chrono::{Duration, Local, NaiveDate, NaiveTime};
use crossterm::{
    cursor,
    event::{self, Event as TermEvent, KeyCode, KeyModifiers},
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode},
};

use super::format::format_event_card;
use crate::{
    completion,
    model::{EventOccurrence, Importance, NewEvent, Tag, parse_date, parse_time},
    storage::Database,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AddArgs {
    pub title: Option<String>,
    pub date: Option<String>,
    pub time: Option<String>,
    pub importance: Option<String>,
    pub tags: Option<String>,
    pub directory: Option<String>,
    pub description: Option<String>,
}

impl AddArgs {
    pub fn parse_from(args: &[String]) -> Self {
        let mut parsed = Self::default();
        let mut iter = args.iter().peekable();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--title" | "-t" => parsed.title = iter.next().cloned(),
                "--date" | "-d" => parsed.date = iter.next().cloned(),
                "--time" => parsed.time = iter.next().cloned(),
                "--importance" | "-i" => parsed.importance = iter.next().cloned(),
                "--tags" => parsed.tags = iter.next().cloned(),
                "--dir" | "--directory" => parsed.directory = iter.next().cloned(),
                "--desc" | "--description" => parsed.description = iter.next().cloned(),
                _ => {}
            }
        }
        parsed
    }
}

pub fn parse_cli_date(input: &str, today: NaiveDate) -> Result<NaiveDate, Box<dyn Error>> {
    let clean = input.trim().to_lowercase();
    if clean.is_empty() || clean == "today" || clean == "сегодня" {
        Ok(today)
    } else if clean == "tomorrow" || clean == "завтра" {
        Ok(today + Duration::days(1))
    } else {
        parse_date(&clean).map_err(|e| format!("неверный формат даты '{input}': {e}").into())
    }
}

pub fn parse_cli_time(
    input: &str,
) -> Result<(Option<NaiveTime>, Option<NaiveTime>), Box<dyn Error>> {
    let clean = input.trim();
    if clean.is_empty() {
        return Ok((None, None));
    }
    if let Some((start_s, end_s)) = clean.split_once('-') {
        let start = parse_time(start_s.trim())?;
        let end = parse_time(end_s.trim())?;
        Ok((Some(start), Some(end)))
    } else {
        let start = parse_time(clean)?;
        Ok((Some(start), None))
    }
}

pub fn parse_cli_importance(input: &str) -> Importance {
    match input.trim().to_lowercase().as_str() {
        "0" | "none" | "нет" => Importance::None,
        "1" | "low" | "низкая" => Importance::Low,
        "3" | "4" | "high" | "высокая" => Importance::High,
        _ => Importance::Normal,
    }
}

pub fn parse_cli_directory(input: &str) -> Result<Option<PathBuf>, Box<dyn Error>> {
    let clean = input.trim();
    if clean.is_empty() {
        return Ok(None);
    }
    let expanded = completion::expand_tilde(clean);
    let canonical = std::fs::canonicalize(&expanded)
        .map_err(|e| format!("директория '{clean}' не найдена: {e}"))?;
    if !canonical.is_dir() {
        return Err(format!("путь '{clean}' не является директорией").into());
    }
    Ok(Some(canonical))
}

pub fn parse_cli_tags(input: &str) -> Vec<String> {
    input
        .split(|c: char| c.is_whitespace() || c == ',')
        .map(|t| t.trim_start_matches('#').trim())
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect()
}

struct InteractiveInputs {
    title: String,
    date: String,
    time: String,
    importance: String,
    tags: String,
    directory: String,
    description: String,
}

type Completer<'a> = &'a dyn Fn(&str) -> Vec<String>;

pub fn run_add(database: &mut Database, args: &AddArgs) -> Result<(), Box<dyn Error>> {
    let today = Local::now().date_naive();

    let inputs = if let Some(title) = &args.title {
        InteractiveInputs {
            title: title.clone(),
            date: args.date.clone().unwrap_or_default(),
            time: args.time.clone().unwrap_or_default(),
            importance: args.importance.clone().unwrap_or_default(),
            tags: args.tags.clone().unwrap_or_default(),
            directory: args.directory.clone().unwrap_or_default(),
            description: args.description.clone().unwrap_or_default(),
        }
    } else {
        prompt_interactive(database, today)?
    };

    let title = inputs.title.trim().to_string();
    if title.is_empty() {
        return Err("название события не может быть пустым".into());
    }

    let date = parse_cli_date(&inputs.date, today)?;
    let (start_time, end_time) = parse_cli_time(&inputs.time)?;
    let importance = parse_cli_importance(&inputs.importance);
    let directory = parse_cli_directory(&inputs.directory)?;
    let tags = parse_cli_tags(&inputs.tags);
    let description = if inputs.description.trim().is_empty() {
        None
    } else {
        Some(inputs.description.trim().to_string())
    };

    let new_event = NewEvent {
        title: title.clone(),
        description: description.clone(),
        start_date: date,
        start_time,
        end_time,
        importance,
        directory: directory.clone(),
    };

    let event_id = database.create_event(&new_event, None, &tags, &[])?;

    let occurrence = EventOccurrence {
        event_id,
        recurrence_id: None,
        original_date: date,
        date,
        start_time,
        end_time,
        title,
        description,
        importance,
        tags: tags
            .iter()
            .map(|t| Tag {
                id: 0,
                name: t.clone(),
                normalized_name: t.clone(),
            })
            .collect(),
        favorite_links: Vec::new(),
        directory,
        is_recurring: false,
    };

    println!("\x1b[1;32m✓ Событие успешно создано!\x1b[0m\n");
    println!("{}", format_event_card(&occurrence));

    Ok(())
}

pub fn prompt_create_event(
    database: &mut Database,
    default_date: NaiveDate,
) -> Result<Option<EventOccurrence>, Box<dyn Error>> {
    let inputs = match prompt_interactive(database, default_date) {
        Ok(inputs) => inputs,
        Err(_) => return Ok(None),
    };

    let title = inputs.title.trim().to_string();
    if title.is_empty() {
        return Ok(None);
    }

    let date = parse_cli_date(&inputs.date, default_date)?;
    let (start_time, end_time) = parse_cli_time(&inputs.time)?;
    let importance = parse_cli_importance(&inputs.importance);
    let directory = parse_cli_directory(&inputs.directory)?;
    let tags = parse_cli_tags(&inputs.tags);
    let description = if inputs.description.trim().is_empty() {
        None
    } else {
        Some(inputs.description.trim().to_string())
    };

    let new_event = NewEvent {
        title: title.clone(),
        description: description.clone(),
        start_date: date,
        start_time,
        end_time,
        importance,
        directory: directory.clone(),
    };

    let event_id = database.create_event(&new_event, None, &tags, &[])?;

    let occurrence = EventOccurrence {
        event_id,
        recurrence_id: None,
        original_date: date,
        date,
        start_time,
        end_time,
        title,
        description,
        importance,
        tags: tags
            .iter()
            .map(|t| Tag {
                id: 0,
                name: t.clone(),
                normalized_name: t.clone(),
            })
            .collect(),
        favorite_links: Vec::new(),
        directory,
        is_recurring: false,
    };

    Ok(Some(occurrence))
}


fn prompt_interactive(
    database: &Database,
    today: NaiveDate,
) -> Result<InteractiveInputs, Box<dyn Error>> {
    println!("\x1b[1;36m=== Создание нового события ===\x1b[0m\n");

    let title = prompt_line("[1/7] Название (обязательно): ", "", None)?;
    if title.trim().is_empty() {
        return Err("название не может быть пустым".into());
    }

    let default_date = today.format("%d.%m.%Y").to_string();
    let date = prompt_line(
        &format!("[2/7] Дата [DD.MM.YYYY, today, tomorrow] (по умолчанию: {default_date}): "),
        &default_date,
        None,
    )?;

    let time = prompt_line(
        "[3/7] Время [HH:MM или HH:MM-HH:MM] (Enter — весь день): ",
        "",
        None,
    )?;

    let importance = prompt_line(
        "[4/7] Важность [none / low / normal / high] (по умолчанию: normal): ",
        "normal",
        None,
    )?;

    let tag_completer = |input: &str| {
        let last_word = input.split_whitespace().next_back().unwrap_or(input);
        completion::complete_tags(database, last_word, 5)
    };

    let tags = prompt_line(
        "[5/7] Теги через пробел (Tab для автодополнения): ",
        "",
        Some(&tag_completer),
    )?;

    let dir_completer = |input: &str| completion::complete_directories(input, 5);

    let directory = prompt_line(
        "[6/7] Директория/папка (Tab для автодополнения): ",
        "",
        Some(&dir_completer),
    )?;

    let description = prompt_line("[7/7] Описание (Enter — без описания): ", "", None)?;

    println!();
    Ok(InteractiveInputs {
        title,
        date,
        time,
        importance,
        tags,
        directory,
        description,
    })
}

fn prompt_line(
    prompt: &str,
    default_val: &str,
    completer: Option<Completer<'_>>,
) -> Result<String, Box<dyn Error>> {
    print!("{prompt}");
    io::stdout().flush()?;

    enable_raw_mode()?;
    let mut buffer = String::new();
    let mut suggestion_idx = 0usize;
    let mut current_suggestions: Vec<String> = Vec::new();

    let res = loop {
        if let TermEvent::Key(key) = event::read()? {
            match (key.code, key.modifiers) {
                (KeyCode::Char('c'), KeyModifiers::CONTROL) | (KeyCode::Esc, _) => {
                    disable_raw_mode()?;
                    println!();
                    return Err("ввод отменен".into());
                }
                (KeyCode::Enter, _) => {
                    let result = if buffer.trim().is_empty() && !default_val.is_empty() {
                        default_val.to_string()
                    } else {
                        buffer.clone()
                    };
                    break Ok(result);
                }
                (KeyCode::Backspace, _) => {
                    buffer.pop();
                    current_suggestions.clear();
                    suggestion_idx = 0;
                    redraw_prompt(prompt, &buffer)?;
                }
                (KeyCode::Tab, KeyModifiers::NONE) => {
                    if let Some(comp) = completer {
                        if current_suggestions.is_empty() {
                            current_suggestions = comp(&buffer);
                            suggestion_idx = 0;
                        } else {
                            suggestion_idx = (suggestion_idx + 1) % current_suggestions.len();
                        }

                        if !current_suggestions.is_empty() {
                            let chosen = &current_suggestions[suggestion_idx];
                            // If completing tags (with prefix space)
                            if buffer.contains(' ') {
                                if let Some(last_space) = buffer.rfind(' ') {
                                    buffer.truncate(last_space + 1);
                                    buffer.push_str(chosen);
                                }
                            } else {
                                buffer = chosen.clone();
                            }
                            redraw_prompt(prompt, &buffer)?;
                        }
                    }
                }
                (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                    buffer.push(c);
                    current_suggestions.clear();
                    suggestion_idx = 0;
                    redraw_prompt(prompt, &buffer)?;
                }
                _ => {}
            }
        }
    };

    disable_raw_mode()?;
    println!();
    res
}

fn redraw_prompt(prompt: &str, buffer: &str) -> Result<(), Box<dyn Error>> {
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        cursor::MoveToColumn(0),
        Clear(ClearType::CurrentLine)
    )?;
    print!("{prompt}{buffer}");
    stdout.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cli_date_keywords() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        assert_eq!(parse_cli_date("today", today).unwrap(), today);
        assert_eq!(parse_cli_date("сегодня", today).unwrap(), today);
        assert_eq!(
            parse_cli_date("tomorrow", today).unwrap(),
            today + Duration::days(1)
        );
        assert_eq!(
            parse_cli_date("завтра", today).unwrap(),
            today + Duration::days(1)
        );
        assert_eq!(
            parse_cli_date("05.09.2026", today).unwrap(),
            NaiveDate::from_ymd_opt(2026, 9, 5).unwrap()
        );
    }

    #[test]
    fn parse_cli_time_ranges() {
        let (s, e) = parse_cli_time("10:00-11:30").unwrap();
        assert_eq!(s, Some(NaiveTime::from_hms_opt(10, 0, 0).unwrap()));
        assert_eq!(e, Some(NaiveTime::from_hms_opt(11, 30, 0).unwrap()));

        let (s, e) = parse_cli_time("14:40").unwrap();
        assert_eq!(s, Some(NaiveTime::from_hms_opt(14, 40, 0).unwrap()));
        assert_eq!(e, None);

        let (s, e) = parse_cli_time("").unwrap();
        assert_eq!(s, None);
        assert_eq!(e, None);
    }

    #[test]
    fn parse_cli_args_parsing() {
        let args = vec![
            "--title".into(),
            "Семинар".into(),
            "--date".into(),
            "02.09.2026".into(),
            "--importance".into(),
            "high".into(),
            "--tags".into(),
            "#универ, #физика".into(),
        ];
        let parsed = AddArgs::parse_from(&args);
        assert_eq!(parsed.title.as_deref(), Some("Семинар"));
        assert_eq!(parsed.date.as_deref(), Some("02.09.2026"));
        assert_eq!(parsed.importance.as_deref(), Some("high"));
        assert_eq!(parsed.tags.as_deref(), Some("#универ, #физика"));
    }

    #[test]
    fn run_add_creates_event_in_db() {
        let mut db = Database::in_memory().unwrap();
        let args = AddArgs {
            title: Some("Коллоквиум по физике".into()),
            date: Some("01.09.2026".into()),
            time: Some("14:40-16:00".into()),
            importance: Some("high".into()),
            tags: Some("#физика, #сессия".into()),
            directory: None,
            description: Some("Повторить формулы".into()),
        };
        run_add(&mut db, &args).unwrap();

        let date = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let events = db.events_between(date, date).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Коллоквиум по физике");
        assert_eq!(events[0].importance, Importance::High);
        assert_eq!(events[0].tags.len(), 2);
    }
}
