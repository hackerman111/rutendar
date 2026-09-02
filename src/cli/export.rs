use std::{
    error::Error,
    path::{Path, PathBuf},
};

use chrono::Local;

use crate::storage::Database;

pub fn run_export(database: &Database, path: Option<&Path>) -> Result<(), Box<dyn Error>> {
    let target_path: PathBuf = match path {
        Some(p) => p.to_path_buf(),
        None => {
            let filename = format!(
                "rutendar-backup-{}.db",
                Local::now().format("%Y-%m-%d-%H%M%S")
            );
            PathBuf::from(filename)
        }
    };

    let (file_size, event_count) = database.export(&target_path)?;
    let size_formatted = if file_size < 1024 {
        format!("{file_size} B")
    } else if file_size < 1024 * 1024 {
        format!("{:.1} KB", file_size as f64 / 1024.0)
    } else {
        format!("{:.1} MB", file_size as f64 / (1024.0 * 1024.0))
    };

    let canonical = target_path.canonicalize().unwrap_or(target_path.clone());
    println!("\x1b[1;32m✓ База данных успешно экспортирована\x1b[0m");
    println!("  Файл: \x1b[1m{}\x1b[0m", canonical.display());
    println!("  Размер: \x1b[36m{}\x1b[0m", size_formatted);
    println!("  Событий сохранено: \x1b[33m{}\x1b[0m", event_count);
    Ok(())
}

pub fn run_export_ics(database: &Database, path: Option<&Path>) -> Result<(), Box<dyn Error>> {
    let target_path: PathBuf = match path {
        Some(p) => p.to_path_buf(),
        None => {
            let filename = format!(
                "rutendar-export-{}.ics",
                Local::now().format("%Y-%m-%d-%H%M%S")
            );
            PathBuf::from(filename)
        }
    };

    if let Some(parent) = target_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let events = database.all_events_for_export()?;
    let tasks = database.all_tasks(crate::model::TaskFilter::All)?;
    let event_count = events.len();
    let task_count = tasks.len();

    let ics_content = crate::ics::format_ics(&events, &tasks);
    std::fs::write(&target_path, ics_content.as_bytes())?;

    let file_size = std::fs::metadata(&target_path)?.len();
    let size_formatted = if file_size < 1024 {
        format!("{file_size} B")
    } else if file_size < 1024 * 1024 {
        format!("{:.1} KB", file_size as f64 / 1024.0)
    } else {
        format!("{:.1} MB", file_size as f64 / (1024.0 * 1024.0))
    };

    let canonical = target_path.canonicalize().unwrap_or(target_path.clone());
    println!("\x1b[1;32m✓ Календарь успешно экспортирован в iCalendar (.ics)\x1b[0m");
    println!("  Файл: \x1b[1m{}\x1b[0m", canonical.display());
    println!("  Размер: \x1b[36m{}\x1b[0m", size_formatted);
    println!("  Событий: \x1b[33m{}\x1b[0m", event_count);
    println!("  Задач: \x1b[35m{}\x1b[0m", task_count);
    println!(
        "  \x1b[90mПодсказка: Для переноса в Google Календарь перейдите в Настройки -> 'Импорт и экспорт' -> 'Импорт' и выберите этот файл.\x1b[0m"
    );
    Ok(())
}
