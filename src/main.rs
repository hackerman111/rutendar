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
    let args: Vec<String> = std::env::args().skip(1).collect();
    let bin_name = std::env::args()
        .next()
        .and_then(|s| {
            std::path::Path::new(&s)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
        })
        .unwrap_or_default();

    let mut cmd = rutendar::cli::parse_cli_command(&args)?;
    if cmd.is_none() && bin_name == "ruty" {
        cmd = Some(rutendar::cli::CliCommand::List(None));
    }

    if let Some(cmd) = cmd {
        let (_config, paths) = Config::load()?;
        match cmd {
            rutendar::cli::CliCommand::List(period) => {
                let database = Database::open(&paths.database)?;
                rutendar::cli::run_list(&database, period)?;
            }
            rutendar::cli::CliCommand::Add(add_args) => {
                let mut database = Database::open(&paths.database)?;
                rutendar::cli::run_add(&mut database, &add_args)?;
            }
            rutendar::cli::CliCommand::Export(target) => {
                let database = Database::open(&paths.database)?;
                rutendar::cli::run_export(&database, target.as_deref())?;
            }
            rutendar::cli::CliCommand::Import { path, force } => {
                rutendar::cli::run_import(&paths.database, &path, force)?;
            }
            rutendar::cli::CliCommand::TaskMenu => {
                let database = Database::open(&paths.database)?;
                rutendar::cli::run_task_menu(&database)?;
            }
            rutendar::cli::CliCommand::TaskAdd(task_args) => {
                let database = Database::open(&paths.database)?;
                rutendar::cli::run_task_add(&database, &task_args)?;
            }
            rutendar::cli::CliCommand::TaskToggle(id) => {
                let database = Database::open(&paths.database)?;
                rutendar::cli::run_task_toggle(&database, id)?;
            }
            rutendar::cli::CliCommand::TaskList(filter) => {
                let database = Database::open(&paths.database)?;
                rutendar::cli::run_task_list(&database, filter.as_deref())?;
            }
            rutendar::cli::CliCommand::NotesMenu => {
                let database = Database::open(&paths.database)?;
                rutendar::cli::run_notes_menu(&database)?;
            }
            rutendar::cli::CliCommand::NotesExport {
                period,
                file,
                stdout,
            } => {
                let database = Database::open(&paths.database)?;
                rutendar::cli::run_notes_export(&database, period, file.as_deref(), stdout)?;
            }
            rutendar::cli::CliCommand::Help => {
                rutendar::cli::print_help();
            }
        }
        return Ok(());
    }

    let (config, paths) = Config::load()?;
    let database = Database::open(&paths.database)?;
    let key_config = config.keys.clone();
    let mut app = App::new(database, config)?;
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
