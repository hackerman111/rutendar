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
