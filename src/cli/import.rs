use std::{
    error::Error,
    io::{self, Write},
    path::Path,
};

use chrono::Local;

use crate::storage::Database;

pub fn run_import(
    current_db_path: &Path,
    import_path: &Path,
    force: bool,
) -> Result<(), Box<dyn Error>> {
    if !import_path.exists() {
        return Err(format!("файл для импорта не найден: {}", import_path.display()).into());
    }

    // Step 1: Validate file integrity and schema
    let event_count = Database::validate_file(import_path)?;

    // Step 2: Confirm if not forced
    if !force {
        println!("\x1b[1;33mВНИМАНИЕ:\x1b[0m Импорт заменит текущую базу данных расписания!");
        println!(
            "  Файл импорта: \x1b[1m{}\x1b[0m (событий: \x1b[33m{}\x1b[0m)",
            import_path.display(),
            event_count
        );
        print!("Вы уверены, что хотите продолжить? [y/N]: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let answer = input.trim().to_lowercase();
        if answer != "y" && answer != "yes" && answer != "д" && answer != "да" {
            println!("Импорт отменен пользователем.");
            return Ok(());
        }
    }

    // Step 3: Backup existing database if it exists
    if current_db_path.exists() {
        let backup_name = format!(
            "{}.bak.{}",
            current_db_path.display(),
            Local::now().format("%Y%m%d_%H%M%S")
        );
        std::fs::copy(current_db_path, &backup_name)?;
        println!("  Создана резервная копия: \x1b[2m{}\x1b[0m", backup_name);
    }

    // Step 4: Ensure parent directory exists
    if let Some(parent) = current_db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Remove active WAL/SHM if they exist
    let wal_path = current_db_path.with_extension("db-wal");
    let shm_path = current_db_path.with_extension("db-shm");
    let _ = std::fs::remove_file(wal_path);
    let _ = std::fs::remove_file(shm_path);

    // Step 5: Copy import file to destination
    std::fs::copy(import_path, current_db_path)?;

    // Step 6: Open and run migrations if needed
    let _db = Database::open(current_db_path)?;

    println!("\x1b[1;32m✓ База данных успешно импортирована!\x1b[0m");
    println!(
        "  Текущая база: \x1b[1m{}\x1b[0m",
        current_db_path.display()
    );
    println!("  Загружено событий: \x1b[33m{}\x1b[0m", event_count);
    Ok(())
}
