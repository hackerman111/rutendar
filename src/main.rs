use std::{error::Error, io, time::Duration};

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use rutendar::{app::App, config::Config, external, input::Keymap, storage::Database, ui};

fn main() {
    if let Err(error) = run() {
        eprintln!("rutendar: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let cli_theme = rutendar::cli::extract_theme_arg(&mut args);
    let cmd = rutendar::cli::parse_cli_command(&args)?;
    let (mut config, paths) = Config::load()?;
    if let Some(theme) = cli_theme {
        config.ui.theme = theme;
    }

    match cmd {
        Some(rutendar::cli::CliCommand::FullTui) => {
            let database = Database::open(&paths.database)?;
            run_fullscreen_tui(database, config, None)
        }
        Some(rutendar::cli::CliCommand::Add(add_args)) => {
            let mut database = Database::open(&paths.database)?;
            rutendar::cli::run_add(&mut database, &add_args)?;
            Ok(())
        }
        Some(rutendar::cli::CliCommand::Export(target)) => {
            let database = Database::open(&paths.database)?;
            rutendar::cli::run_export(&database, target.as_deref())?;
            Ok(())
        }
        Some(rutendar::cli::CliCommand::ExportIcs(target)) => {
            let database = Database::open(&paths.database)?;
            rutendar::cli::run_export_ics(&database, target.as_deref())?;
            Ok(())
        }
        Some(rutendar::cli::CliCommand::Import { path, force }) => {
            rutendar::cli::run_import(&paths.database, &path, force)?;
            Ok(())
        }
        Some(rutendar::cli::CliCommand::ImportIcs { path }) => {
            let mut database = Database::open(&paths.database)?;
            rutendar::cli::run_import_ics(&mut database, &path)?;
            Ok(())
        }
        Some(rutendar::cli::CliCommand::TaskMenu) => {
            let database = Database::open(&paths.database)?;
            rutendar::cli::run_task_menu(&database)?;
            Ok(())
        }
        Some(rutendar::cli::CliCommand::TaskAdd(task_args)) => {
            let database = Database::open(&paths.database)?;
            rutendar::cli::run_task_add(&database, &task_args)?;
            Ok(())
        }
        Some(rutendar::cli::CliCommand::TaskToggle(id)) => {
            let database = Database::open(&paths.database)?;
            rutendar::cli::run_task_toggle(&database, id)?;
            Ok(())
        }
        Some(rutendar::cli::CliCommand::TaskList(filter)) => {
            let database = Database::open(&paths.database)?;
            rutendar::cli::run_task_list(&database, filter.as_deref())?;
            Ok(())
        }
        Some(rutendar::cli::CliCommand::NotesMenu) => {
            let database = Database::open(&paths.database)?;
            rutendar::cli::run_notes_menu(&database)?;
            Ok(())
        }
        Some(rutendar::cli::CliCommand::NotesExport {
            period,
            date,
            file,
            stdout,
        }) => {
            let database = Database::open(&paths.database)?;
            rutendar::cli::run_notes_export(&database, period, date, file.as_deref(), stdout)?;
            Ok(())
        }
        Some(rutendar::cli::CliCommand::Help) => {
            rutendar::cli::print_help();
            Ok(())
        }
        Some(rutendar::cli::CliCommand::Inline(period))
        | Some(rutendar::cli::CliCommand::List(period)) => {
            let mut database = Database::open(&paths.database)?;
            let outcome = rutendar::cli::run_inline(&mut database, &config, period)?;
            match outcome {
                rutendar::cli::InlineOutcome::Exit => Ok(()),
                rutendar::cli::InlineOutcome::OpenFullTui { initial_date } => {
                    run_fullscreen_tui(database, config, initial_date)
                }
            }
        }
        None => {
            let mut database = Database::open(&paths.database)?;
            let outcome = rutendar::cli::run_inline(&mut database, &config, None)?;
            match outcome {
                rutendar::cli::InlineOutcome::Exit => Ok(()),
                rutendar::cli::InlineOutcome::OpenFullTui { initial_date } => {
                    run_fullscreen_tui(database, config, initial_date)
                }
            }
        }
    }
}

fn run_fullscreen_tui(
    database: Database,
    config: Config,
    initial_date: Option<chrono::NaiveDate>,
) -> Result<(), Box<dyn Error>> {
    let key_config = config.keys.clone();
    let mut app = App::new(database, config)?;
    if let Some(date) = initial_date {
        let _ = app.select_date(date);
    }
    let mut keymap = Keymap::new(&key_config);
    let mut tui = Tui::new()?;
    event_loop(&mut tui, &mut app, &mut keymap)
}

fn event_loop(tui: &mut Tui, app: &mut App, keymap: &mut Keymap) -> Result<(), Box<dyn Error>> {
    loop {
        tui.terminal.draw(|frame| ui::render(frame, app))?;
        app.tick();
        if event::poll(Duration::from_millis(200))?
            && let Event::Key(key) = event::read()?
        {
            let action = keymap.map(key, app.input_mode());
            if app.update(action) {
                return Ok(());
            }
            if let Some(directory) = app.take_directory_request() {
                tui.suspend()?;
                let shell_result = external::open_shell(&directory);
                let resume_result = tui.resume();
                if let Err(error) = shell_result {
                    app.report_error(error);
                }
                resume_result?;
            }
        }
    }
}

struct Tui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    active: bool,
}

impl Tui {
    fn new() -> Result<Self, Box<dyn Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen) {
            _ = disable_raw_mode();
            return Err(error.into());
        }
        match Terminal::new(CrosstermBackend::new(stdout)) {
            Ok(terminal) => Ok(Self {
                terminal,
                active: true,
            }),
            Err(error) => {
                let mut stdout = io::stdout();
                _ = execute!(stdout, LeaveAlternateScreen);
                _ = disable_raw_mode();
                Err(error.into())
            }
        }
    }

    fn suspend(&mut self) -> Result<(), Box<dyn Error>> {
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        self.active = false;
        Ok(())
    }

    fn resume(&mut self) -> Result<(), Box<dyn Error>> {
        execute!(self.terminal.backend_mut(), EnterAlternateScreen)?;
        self.active = true;
        enable_raw_mode()?;
        self.terminal.clear()?;
        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        _ = disable_raw_mode();
        if self.active {
            _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        }
        _ = self.terminal.show_cursor();
    }
}
