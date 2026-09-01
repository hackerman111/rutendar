pub mod add;
pub mod export;
pub mod format;
pub mod import;
pub mod list;
pub mod task;

use std::path::PathBuf;

pub use add::{AddArgs, run_add};
pub use export::run_export;
pub use format::format_event_card;
pub use import::run_import;
pub use list::{Period, run_list};
pub use task::{TaskAddArgs, run_task_add, run_task_list, run_task_menu, run_task_toggle};

#[derive(Debug, PartialEq, Eq)]
pub enum CliCommand {
    List(Option<Period>),
    Add(AddArgs),
    Export(Option<PathBuf>),
    Import { path: PathBuf, force: bool },
    TaskMenu,
    TaskAdd(TaskAddArgs),
    TaskToggle(i64),
    TaskList(Option<String>),
    Help,
}

pub fn parse_cli_command(args: &[String]) -> Result<Option<CliCommand>, String> {
    let Some(first) = args.first() else {
        return Ok(None);
    };
    match first.as_str() {
        "--list" | "-l" | "list" => {
            let period = args.get(1).and_then(|s| Period::parse(s));
            Ok(Some(CliCommand::List(period)))
        }
        "--add" | "-a" | "add" => {
            let add_args = AddArgs::parse_from(&args[1..]);
            Ok(Some(CliCommand::Add(add_args)))
        }
        "--export" | "-e" | "export" => {
            let path = args.get(1).map(PathBuf::from);
            Ok(Some(CliCommand::Export(path)))
        }
        "--import" | "-i" | "import" => {
            let mut path = None;
            let mut force = false;
            for arg in &args[1..] {
                if arg == "--force" || arg == "-f" {
                    force = true;
                } else if path.is_none() {
                    path = Some(PathBuf::from(arg));
                }
            }
            let Some(path) = path else {
                return Err(
                    "укажите путь к файлу базы данных для импорта: rutendar --import <ФАЙЛ>".into(),
                );
            };
            Ok(Some(CliCommand::Import { path, force }))
        }
        "--task" | "-t" | "task" => {
            let sub = args.get(1).map(|s| s.as_str());
            match sub {
                Some("--add" | "-a" | "add") => {
                    let task_args = TaskAddArgs::parse_from(&args[2..]);
                    Ok(Some(CliCommand::TaskAdd(task_args)))
                }
                Some("--toggle" | "--done" | "done" | "toggle") => {
                    let id = args
                        .get(2)
                        .and_then(|s| s.parse::<i64>().ok())
                        .ok_or("укажите числовой ID задания: rutendar --task --done <ID>")?;
                    Ok(Some(CliCommand::TaskToggle(id)))
                }
                Some("--list" | "-l" | "list") => {
                    let filter = args.get(2).cloned();
                    Ok(Some(CliCommand::TaskList(filter)))
                }
                None => Ok(Some(CliCommand::TaskMenu)),
                Some(_) => {
                    let task_args = TaskAddArgs::parse_from(&args[1..]);
                    Ok(Some(CliCommand::TaskAdd(task_args)))
                }
            }
        }
        "--task-add" => {
            let task_args = TaskAddArgs::parse_from(&args[1..]);
            Ok(Some(CliCommand::TaskAdd(task_args)))
        }
        "--task-toggle" | "--task-done" => {
            let id = args
                .get(1)
                .and_then(|s| s.parse::<i64>().ok())
                .ok_or("укажите числовой ID задания: rutendar --task-toggle <ID>")?;
            Ok(Some(CliCommand::TaskToggle(id)))
        }
        "--task-list" => {
            let filter = args.get(1).cloned();
            Ok(Some(CliCommand::TaskList(filter)))
        }
        "--help" | "-h" | "help" => Ok(Some(CliCommand::Help)),
        unknown if unknown.starts_with('-') => Err(format!(
            "неизвестный флаг: '{unknown}'. Используйте --help для справки."
        )),
        unknown => {
            if let Some(period) = Period::parse(unknown) {
                Ok(Some(CliCommand::List(Some(period))))
            } else {
                Err(format!(
                    "неизвестный аргумент: '{unknown}'. Используйте --help для справки."
                ))
            }
        }
    }
}

pub fn print_help() {
    println!("\x1b[1;36mRutendar\x1b[0m — локальный терминальный календарь\n");
    println!("ИСПОЛЬЗОВАНИЕ:");
    println!("  rutendar                          Запуск полноэкранного TUI календаря");
    println!(
        "  rutendar --list [day|week|month]  Интерактивный просмотр и поиск ближайших событий (-l)"
    );
    println!("  rutendar --add [опции]            Добавление нового события из терминала (-a)");
    println!("  rutendar --task [опции]           Интерактивное меню заданий (To-Do) (-t)");
    println!("  rutendar --task-add [опции]       Добавление нового задания");
    println!("  rutendar --task-toggle <ID>       Переключение статуса задания (--task-done)");
    println!("  rutendar --task-list [today|all]  Вывод списка заданий в терминал");
    println!("  rutendar --export [файл]          Экспорт базы данных в файл (-e)");
    println!("  rutendar --import <файл> [-f]     Импорт базы данных из файла (-i)");
    println!("  rutendar --help                   Вывод этой справки (-h)\n");
    println!("ОПЦИИ ДЛЯ --add:");
    println!("  --title, -t <НАЗВАНИЕ>      Название события");
    println!("  --date, -d <ДАТА>           Дата (DD.MM.YYYY, today, tomorrow)");
    println!("  --time <ВРЕМЯ>              Время (HH:MM или HH:MM-HH:MM)");
    println!("  --importance, -i <ВАЖНОСТЬ> Важность (none, low, normal, high)");
    println!("  --tags <ТЕГИ>               Теги через запятую или пробел (#универ)");
    println!("  --dir <ПАПКА>               Путь к директории на диске");
    println!("  --desc <ОПИСАНИЕ>           Описание события\n");
    println!("ОПЦИИ ДЛЯ --task-add:");
    println!("  --title, -t <НАЗВАНИЕ>      Название задания");
    println!("  --date, -d <ДАТА>           Срок (DD.MM.YYYY, today, tomorrow)");
    println!("  --importance, -i <ВАЖНОСТЬ> Важность (none, low, normal, high)");
    println!("  --desc <ОПИСАНИЕ>           Описание задания\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cli_commands() {
        assert_eq!(parse_cli_command(&[]), Ok(None));

        assert_eq!(
            parse_cli_command(&["--list".into()]),
            Ok(Some(CliCommand::List(None)))
        );
        assert_eq!(
            parse_cli_command(&["-l".into(), "month".into()]),
            Ok(Some(CliCommand::List(Some(Period::Month))))
        );
        assert_eq!(
            parse_cli_command(&["week".into()]),
            Ok(Some(CliCommand::List(Some(Period::Week))))
        );

        let add = parse_cli_command(&["--add".into(), "--title".into(), "Test".into()]).unwrap();
        if let Some(CliCommand::Add(args)) = add {
            assert_eq!(args.title.as_deref(), Some("Test"));
        } else {
            panic!("expected Add command");
        }

        assert_eq!(
            parse_cli_command(&["--export".into()]),
            Ok(Some(CliCommand::Export(None)))
        );
        assert_eq!(
            parse_cli_command(&["-e".into(), "my_backup.db".into()]),
            Ok(Some(CliCommand::Export(Some(PathBuf::from(
                "my_backup.db"
            )))))
        );

        assert_eq!(
            parse_cli_command(&["--import".into(), "backup.db".into()]),
            Ok(Some(CliCommand::Import {
                path: PathBuf::from("backup.db"),
                force: false
            }))
        );
        assert_eq!(
            parse_cli_command(&["-i".into(), "backup.db".into(), "-f".into()]),
            Ok(Some(CliCommand::Import {
                path: PathBuf::from("backup.db"),
                force: true
            }))
        );
        assert!(parse_cli_command(&["--import".into()]).is_err());

        assert_eq!(
            parse_cli_command(&["--help".into()]),
            Ok(Some(CliCommand::Help))
        );

        assert_eq!(
            parse_cli_command(&["--task".into()]),
            Ok(Some(CliCommand::TaskMenu))
        );
        assert_eq!(
            parse_cli_command(&["-t".into()]),
            Ok(Some(CliCommand::TaskMenu))
        );

        let task_add = parse_cli_command(&[
            "--task-add".into(),
            "Купить молоко".into(),
            "--date".into(),
            "today".into(),
        ])
        .unwrap();
        if let Some(CliCommand::TaskAdd(args)) = task_add {
            assert_eq!(args.title.as_deref(), Some("Купить молоко"));
            assert_eq!(args.date.as_deref(), Some("today"));
        } else {
            panic!("expected TaskAdd");
        }

        assert_eq!(
            parse_cli_command(&["--task-toggle".into(), "42".into()]),
            Ok(Some(CliCommand::TaskToggle(42)))
        );
        assert_eq!(
            parse_cli_command(&["--task-list".into(), "done".into()]),
            Ok(Some(CliCommand::TaskList(Some("done".into()))))
        );

        assert!(parse_cli_command(&["--unknown".into()]).is_err());
    }

    #[test]
    fn test_task_cli_flow() {
        let db = crate::storage::Database::in_memory().unwrap();
        let add_args = TaskAddArgs {
            title: Some("CLI Задание".into()),
            date: Some("today".into()),
            desc: Some("Описание".into()),
            importance: Some("high".into()),
        };
        run_task_add(&db, &add_args).unwrap();

        let tasks = db.all_tasks(crate::model::TaskFilter::Active).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "CLI Задание");
        assert_eq!(tasks[0].importance, crate::model::Importance::High);

        run_task_toggle(&db, tasks[0].id).unwrap();
        let active = db.all_tasks(crate::model::TaskFilter::Active).unwrap();
        assert!(active.is_empty());
        let done = db.all_tasks(crate::model::TaskFilter::Done).unwrap();
        assert_eq!(done.len(), 1);

        run_task_list(&db, Some("done")).unwrap();
    }

    #[test]
    fn test_export_and_import_cli_flow() {
        use crate::model::NewEvent;
        use chrono::NaiveDate;

        let temp_dir = std::env::temp_dir().join("rutendar_test_cli_export_import");
        let _ = std::fs::create_dir_all(&temp_dir);
        let src_db_path = temp_dir.join("source.db");
        let export_path = temp_dir.join("exported.db");
        let dest_db_path = temp_dir.join("destination.db");

        let _ = std::fs::remove_file(&src_db_path);
        let _ = std::fs::remove_file(&export_path);
        let _ = std::fs::remove_file(&dest_db_path);

        let mut database = crate::storage::Database::open(&src_db_path).unwrap();
        database
            .create_event(
                &NewEvent {
                    title: "Импортное событие".into(),
                    description: Some("Тест".into()),
                    start_date: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
                    start_time: None,
                    end_time: None,
                    importance: crate::model::Importance::Normal,
                    directory: None,
                },
                None,
                &[],
                &[],
            )
            .unwrap();

        // Export via CLI function
        run_export(&database, Some(&export_path)).unwrap();
        assert!(export_path.exists());

        // Import via CLI function into new destination
        run_import(&dest_db_path, &export_path, true).unwrap();
        assert!(dest_db_path.exists());

        // Verify imported database has the event
        let dest_db = crate::storage::Database::open(&dest_db_path).unwrap();
        let events = dest_db
            .events_between(
                NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            )
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Импортное событие");

        // Cleanup
        let _ = std::fs::remove_file(&src_db_path);
        let _ = std::fs::remove_file(&export_path);
        let _ = std::fs::remove_file(&dest_db_path);
        let _ = std::fs::remove_dir(&temp_dir);
    }
}
