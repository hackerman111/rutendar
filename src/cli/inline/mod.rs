pub mod add_form;
pub mod render;
pub mod state;

use std::{error::Error, io, time::Duration};

use chrono::Local;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, TerminalOptions, Viewport, backend::CrosstermBackend};

pub use add_form::{
    AddFormApp, AddFormField, TaskFormApp, TaskFormField, render_add_form, render_task_form,
    run_add_form_interactive, run_add_task_interactive, run_edit_form_interactive,
};
pub use render::render_inline;
pub use state::{InlineApp, InlineOutcome, InlineTab, SelectedDayItem};

use crate::{
    cli::{
        Period,
        format::{format_day_summary, format_event_card},
    },
    config::Config,
    storage::Database,
};

pub fn run_inline(
    database: &mut Database,
    _config: &Config,
    initial_period: Option<Period>,
) -> Result<InlineOutcome, Box<dyn Error>> {
    let today = Local::now().date_naive();
    let initial_tab = match initial_period {
        Some(Period::Week) => InlineTab::Week,
        Some(Period::Day) => InlineTab::Day,
        _ => InlineTab::Day,
    };

    let mut app = InlineApp::new(today, initial_tab);
    app.reload_all(database)?;

    enable_raw_mode()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(13),
        },
    )?;

    let mut card_to_print = None;
    let mut summary_to_print = None;
    let mut outcome = InlineOutcome::Exit;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            render_inline(frame, area, &app);
        })?;

        if event::poll(Duration::from_millis(200))?
            && let Event::Key(key) = event::read()?
        {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                outcome = InlineOutcome::Exit;
                break;
            }

            // If delete confirmation is pending:
            if app.pending_delete.is_some() {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        app.confirm_delete(database)?;
                    }
                    _ => {
                        app.cancel_delete();
                    }
                }
                continue;
            }

            if app.tab == InlineTab::Search {
                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) => {
                        if !app.query.is_empty() {
                            app.search_clear();
                        } else {
                            app.switch_tab(InlineTab::Day);
                        }
                    }
                    (KeyCode::Backspace, _) => {
                        app.search_pop_char();
                    }
                    (KeyCode::Enter, _) if !app.search_results.is_empty() => {
                        card_to_print = Some(app.search_results[app.selected_idx].clone());
                        break;
                    }
                    (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                        if let Some(event) = app.selected_event() {
                            let event_to_edit = event.clone();
                            terminal.clear()?;
                            disable_raw_mode()?;

                            let _ = run_edit_form_interactive(database, &event_to_edit);

                            enable_raw_mode()?;
                            app.reload_all(database)?;
                        }
                    }
                    (KeyCode::Char('x') | KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                        app.request_delete();
                    }
                    (KeyCode::Tab, KeyModifiers::NONE) => {
                        app.cycle_tab();
                    }
                    (KeyCode::BackTab, _) => {
                        app.cycle_tab_prev();
                    }
                    (KeyCode::Up, _) => {
                        app.select_prev();
                    }
                    (KeyCode::Down, _) => {
                        app.select_next();
                    }
                    (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                        app.search_push_char(c);
                    }
                    _ => {}
                }
            } else {
                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) | (KeyCode::Char('q'), KeyModifiers::NONE) => {
                        outcome = InlineOutcome::Exit;
                        break;
                    }
                    (KeyCode::Tab, KeyModifiers::NONE) => {
                        app.cycle_tab();
                    }
                    (KeyCode::BackTab, _) => {
                        app.cycle_tab_prev();
                    }
                    (KeyCode::Char('1'), KeyModifiers::NONE) => {
                        app.switch_tab(InlineTab::Day);
                    }
                    (KeyCode::Char('2'), KeyModifiers::NONE) => {
                        app.switch_tab(InlineTab::Week);
                    }
                    (
                        KeyCode::Char('3') | KeyCode::Char('/') | KeyCode::Char('s'),
                        KeyModifiers::NONE,
                    ) => {
                        app.switch_tab(InlineTab::Search);
                    }
                    (KeyCode::Left | KeyCode::Char('h'), _) if app.tab == InlineTab::Day => {
                        app.prev_day(database)?;
                    }
                    (KeyCode::Right | KeyCode::Char('l'), _) if app.tab == InlineTab::Day => {
                        app.next_day(database)?;
                    }
                    (KeyCode::Char('t'), KeyModifiers::NONE) if app.tab == InlineTab::Day => {
                        app.jump_to_today(database)?;
                    }
                    (KeyCode::Up | KeyCode::Char('k'), _) => {
                        app.select_prev();
                    }
                    (KeyCode::Down | KeyCode::Char('j'), _) => {
                        app.select_next();
                    }
                    (KeyCode::Char(' '), _) if app.tab == InlineTab::Day => {
                        app.toggle_selected_task(database)?;
                    }
                    (KeyCode::Enter, _) => match app.tab {
                        InlineTab::Day => match app.selected_day_item() {
                            Some(SelectedDayItem::Event(e)) => {
                                card_to_print = Some(e.clone());
                                break;
                            }
                            Some(SelectedDayItem::Task(_t)) => {
                                let _ = app.toggle_selected_task(database);
                            }
                            None => {}
                        },
                        InlineTab::Week => {
                            if !app.week_events.is_empty() {
                                card_to_print = Some(app.week_events[app.selected_idx].clone());
                                break;
                            }
                        }
                        InlineTab::Search => {}
                    },
                    (KeyCode::Char('p'), KeyModifiers::NONE) => {
                        summary_to_print = Some((
                            app.current_date,
                            app.day_events.clone(),
                            app.day_tasks.clone(),
                        ));
                        break;
                    }
                    (KeyCode::Char('F'), _) | (KeyCode::Char('T'), KeyModifiers::SHIFT) => {
                        outcome = InlineOutcome::OpenFullTui {
                            initial_date: Some(app.current_date),
                        };
                        break;
                    }
                    (KeyCode::Char('e'), KeyModifiers::NONE) => {
                        if let Some(event) = app.selected_event() {
                            let event_to_edit = event.clone();
                            terminal.clear()?;
                            disable_raw_mode()?;

                            let _ = run_edit_form_interactive(database, &event_to_edit);

                            enable_raw_mode()?;
                            app.reload_all(database)?;
                        }
                    }
                    (KeyCode::Char('x'), KeyModifiers::NONE) => {
                        app.request_delete();
                    }
                    (KeyCode::Char('A'), _) | (KeyCode::Char('+'), _) => {
                        terminal.clear()?;
                        disable_raw_mode()?;

                        let _ = run_add_task_interactive(database, app.current_date);

                        enable_raw_mode()?;
                        app.reload_all(database)?;
                    }
                    (KeyCode::Char('a'), KeyModifiers::NONE) => {
                        terminal.clear()?;
                        disable_raw_mode()?;

                        if app.is_task_selected() {
                            let _ = run_add_task_interactive(database, app.current_date);
                        } else {
                            let _ = run_add_form_interactive(database, app.current_date);
                        }

                        enable_raw_mode()?;
                        app.reload_all(database)?;
                    }

                    _ => {}
                }
            }
        }
    }

    terminal.clear()?;
    disable_raw_mode()?;

    if let Some(event) = card_to_print {
        println!("{}", format_event_card(&event));
    } else if let Some((date, events, tasks)) = summary_to_print {
        println!("{}", format_day_summary(date, &events, &tasks));
    }

    Ok(outcome)
}
