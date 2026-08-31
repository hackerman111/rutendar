use std::{error::Error, io, time::Duration};

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use rutendar::{app::App, config::Config, input::Keymap, storage::Database, ui};

fn main() {
    if let Err(error) = run() {
        eprintln!("rutendar: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let (config, paths) = Config::load()?;
    let database = Database::open(&paths.database)?;
    let key_config = config.keys.clone();
    let mut app = App::new(database, config)?;
    let mut keymap = Keymap::new(&key_config);
    let mut tui = Tui::new()?;
    event_loop(&mut tui.terminal, &mut app, &mut keymap)
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    keymap: &mut Keymap,
) -> Result<(), Box<dyn Error>> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;
        app.tick();
        if event::poll(Duration::from_millis(200))?
            && let Event::Key(key) = event::read()?
        {
            let action = keymap.map(key, app.input_mode());
            if app.update(action) {
                return Ok(());
            }
        }
    }
}

struct Tui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
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
            Ok(terminal) => Ok(Self { terminal }),
            Err(error) => {
                let mut stdout = io::stdout();
                _ = execute!(stdout, LeaveAlternateScreen);
                _ = disable_raw_mode();
                Err(error.into())
            }
        }
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        _ = disable_raw_mode();
        _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        _ = self.terminal.show_cursor();
    }
}
