pub mod add;
pub mod format;
pub mod list;

pub use add::{AddArgs, run_add};
pub use format::format_event_card;
pub use list::{Period, run_list};

#[derive(Debug, PartialEq, Eq)]
pub enum CliCommand {
    List(Option<Period>),
    Add(AddArgs),
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
    println!("  rutendar --help                   Вывод этой справки (-h)\n");
    println!("ОПЦИИ ДЛЯ --add:");
    println!("  --title, -t <НАЗВАНИЕ>      Название события");
    println!("  --date, -d <ДАТА>           Дата (DD.MM.YYYY, today, tomorrow)");
    println!("  --time <ВРЕМЯ>              Время (HH:MM или HH:MM-HH:MM)");
    println!("  --importance, -i <ВАЖНОСТЬ> Важность (none, low, normal, high)");
    println!("  --tags <ТЕГИ>               Теги через запятую или пробел (#универ)");
    println!("  --dir <ПАПКА>               Путь к директории на диске");
    println!("  --desc <ОПИСАНИЕ>           Описание события\n");
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
            parse_cli_command(&["--help".into()]),
            Ok(Some(CliCommand::Help))
        );

        assert!(parse_cli_command(&["--unknown".into()]).is_err());
    }
}
