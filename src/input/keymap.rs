use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::{
    app::{Action, InputMode, View},
    config::KeyConfig,
};

pub struct Keymap {
    open_link: char,
    copy_link: char,
    pending_g: bool,
}

impl Keymap {
    pub fn new(config: &KeyConfig) -> Self {
        Self {
            open_link: config.open_link,
            copy_link: config.copy_link,
            pending_g: false,
        }
    }

    pub fn map(&mut self, key: KeyEvent, mode: InputMode) -> Action {
        if key.kind != KeyEventKind::Press {
            return Action::Noop;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Action::Quit;
        }
        match mode {
            InputMode::Editor => self.editor(key),
            InputMode::Search => self.search(key),
            InputMode::LinkBank => self.link_bank(key),
            InputMode::LinkSearch => self.search(key),
            InputMode::GotoDate => self.goto_date(key),
            InputMode::CreateTask => self.create_task(key),
            InputMode::Confirm => match key.code {
                KeyCode::Enter | KeyCode::Char('y' | 'Y' | 'д' | 'Д' | 'x' | 'X') => {
                    Action::Confirm(true)
                }
                KeyCode::Esc | KeyCode::Char('n' | 'N' | 'н' | 'Н' | 'q') => {
                    Action::Confirm(false)
                }
                _ => Action::Noop,
            },
            InputMode::Scope => match key.code {
                KeyCode::Char('o' | 'O' | 'т' | 'Т' | 'j') | KeyCode::Down => {
                    Action::ChooseOccurrence
                }
                KeyCode::Char('s' | 'S' | 'с' | 'С' | 'k') | KeyCode::Up => Action::ChooseSeries,
                KeyCode::Esc | KeyCode::Char('q') => Action::Back,
                _ => Action::Noop,
            },
            InputMode::Normal => self.normal(key),
        }
    }

    fn editor(&self, key: KeyEvent) -> Action {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('s') => Action::Submit,
                KeyCode::Char('j') | KeyCode::Char('n') => Action::NextField,
                KeyCode::Char('k') | KeyCode::Char('p') => Action::PreviousField,
                KeyCode::Char('h') => Action::AdjustLeft,
                KeyCode::Char('l') => Action::OpenLinkBank,
                _ => Action::Noop,
            };
        }
        match key.code {
            KeyCode::Esc => Action::Back,
            KeyCode::Enter => Action::EnterField,
            KeyCode::Tab => Action::TabField,
            KeyCode::Down => Action::NextField,
            KeyCode::BackTab | KeyCode::Up => Action::PreviousField,
            KeyCode::Left => Action::AdjustLeft,
            KeyCode::Right => Action::AdjustRight,
            KeyCode::Backspace => Action::Backspace,
            KeyCode::Char(character) => Action::Input(character),
            _ => Action::Noop,
        }
    }

    fn link_bank(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => Action::Back,
            KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
            KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
            KeyCode::Enter | KeyCode::Char(' ') => Action::ToggleFavoriteLink,
            KeyCode::Char('/') => Action::StartLinkSearch,
            KeyCode::Char('a') => Action::AddFavoriteLink,
            KeyCode::Char('e') => Action::EditFavoriteLink,
            KeyCode::Char(character) if character == self.open_link => Action::OpenFavoriteLink,
            _ => Action::Noop,
        }
    }

    fn search(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => Action::Back,
            KeyCode::Enter => Action::Submit,
            KeyCode::Backspace => Action::Backspace,
            KeyCode::Char(character) => Action::Input(character),
            _ => Action::Noop,
        }
    }

    fn goto_date(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => Action::Back,
            KeyCode::Enter => Action::Submit,
            KeyCode::Backspace => Action::Backspace,
            KeyCode::Char(character) => Action::Input(character),
            _ => Action::Noop,
        }
    }

    fn create_task(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => Action::Back,
            KeyCode::Enter => Action::Submit,
            KeyCode::Backspace => Action::Backspace,
            KeyCode::Char(character) => Action::Input(character),
            _ => Action::Noop,
        }
    }

    fn normal(&mut self, key: KeyEvent) -> Action {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('d') => Action::PageDown,
                KeyCode::Char('u') => Action::PageUp,
                KeyCode::Char('f') => Action::PageDown,
                KeyCode::Char('b') => Action::PageUp,
                _ => Action::Noop,
            };
        }
        if self.pending_g {
            self.pending_g = false;
            return match key.code {
                KeyCode::Char('g') => Action::GoToTop,
                KeyCode::Char('t') => Action::GoToToday,
                KeyCode::Char('d') => Action::StartGotoDate,
                _ => Action::Noop,
            };
        }
        match key.code {
            KeyCode::Char('g') => {
                self.pending_g = true;
                Action::Noop
            }
            KeyCode::Char('G') | KeyCode::End => Action::GoToBottom,
            KeyCode::Home => Action::GoToTop,
            KeyCode::PageDown => Action::PageDown,
            KeyCode::PageUp => Action::PageUp,
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('h') | KeyCode::Left => Action::MoveLeft,
            KeyCode::Char('l') | KeyCode::Right => Action::MoveRight,
            KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
            KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
            KeyCode::Enter => Action::Open,
            KeyCode::Esc => Action::Back,
            KeyCode::Char('a') => Action::Create,
            KeyCode::Char('n') => Action::NextDay,
            KeyCode::Char('N') => Action::PreviousDay,
            KeyCode::Char('e' | 'r') => Action::Edit,
            KeyCode::Char('d') | KeyCode::Char('x') => Action::Delete,
            KeyCode::Char('X') => Action::DeleteTag,
            KeyCode::Char('/') => Action::OpenAgenda,
            KeyCode::Char('t') => Action::OpenUpcoming,
            KeyCode::Char('T') => Action::StartCreateTask,
            KeyCode::Char('p') => Action::ChangeImportance,
            KeyCode::Char('w') => Action::SwitchView(View::Week),
            KeyCode::Char('D') => Action::SwitchView(View::Day),
            KeyCode::Char('m') => Action::SwitchView(View::Month),
            KeyCode::Char('Y') => Action::SwitchView(View::Year),
            KeyCode::Char('?') => Action::Help,
            KeyCode::Char('c') => Action::OpenDirectory,
            KeyCode::Tab => Action::NextView,
            KeyCode::BackTab => Action::PreviousView,
            KeyCode::Char('f') => Action::CycleDateFilter,
            KeyCode::Char('R') => Action::CycleItemType,
            KeyCode::Char('i') => Action::CycleImportanceFilter,
            KeyCode::Char('s') => Action::CycleSort,
            KeyCode::Char('A') => Action::ToggleTagMatching,
            KeyCode::Char('[') => Action::PreviousTagFilter,
            KeyCode::Char(']') => Action::NextTagFilter,
            KeyCode::Char(' ') => Action::ToggleTagFilter,
            KeyCode::Char(character) if character == self.open_link => Action::OpenLink,
            KeyCode::Char(character) if character == self.copy_link => Action::CopyLink,
            _ => Action::Noop,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g_prefix_maps_to_semantic_actions() {
        let mut keymap = Keymap::new(&KeyConfig::default());
        let key = |character| KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE);
        let ctrl_key = |character| KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL);

        assert!(matches!(
            keymap.map(key('g'), InputMode::Normal),
            Action::Noop
        ));
        assert!(matches!(
            keymap.map(key('t'), InputMode::Normal),
            Action::GoToToday
        ));
        assert!(matches!(
            keymap.map(key('g'), InputMode::Normal),
            Action::Noop
        ));
        assert!(matches!(
            keymap.map(key('g'), InputMode::Normal),
            Action::GoToTop
        ));
        assert!(matches!(
            keymap.map(key('g'), InputMode::Normal),
            Action::Noop
        ));
        assert!(matches!(
            keymap.map(key('d'), InputMode::Normal),
            Action::StartGotoDate
        ));
        assert!(matches!(
            keymap.map(key('G'), InputMode::Normal),
            Action::GoToBottom
        ));
        assert!(matches!(
            keymap.map(ctrl_key('d'), InputMode::Normal),
            Action::PageDown
        ));
        assert!(matches!(
            keymap.map(ctrl_key('u'), InputMode::Normal),
            Action::PageUp
        ));
        assert!(matches!(
            keymap.map(
                KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
                InputMode::Normal
            ),
            Action::NextView
        ));
        assert!(matches!(
            keymap.map(
                KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE),
                InputMode::Normal
            ),
            Action::PreviousView
        ));
        assert!(matches!(
            keymap.map(key('x'), InputMode::Normal),
            Action::Delete
        ));
        assert!(matches!(
            keymap.map(key('d'), InputMode::Normal),
            Action::Delete
        ));
        assert!(matches!(
            keymap.map(key('X'), InputMode::Normal),
            Action::DeleteTag
        ));
        assert!(matches!(
            keymap.map(key('a'), InputMode::Normal),
            Action::Create
        ));
        assert!(matches!(
            keymap.map(key('n'), InputMode::Normal),
            Action::NextDay
        ));
        assert!(matches!(
            keymap.map(key('N'), InputMode::Normal),
            Action::PreviousDay
        ));
        assert!(matches!(
            keymap.map(key('/'), InputMode::Normal),
            Action::OpenAgenda
        ));
        assert!(matches!(
            keymap.map(key('c'), InputMode::Normal),
            Action::OpenDirectory
        ));
        assert!(matches!(
            keymap.map(
                KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
                InputMode::Editor
            ),
            Action::EnterField
        ));
        assert!(matches!(
            keymap.map(
                KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
                InputMode::Editor
            ),
            Action::TabField
        ));
        assert!(matches!(
            keymap.map(key('a'), InputMode::LinkBank),
            Action::AddFavoriteLink
        ));
        assert!(matches!(
            keymap.map(key('e'), InputMode::Normal),
            Action::Edit
        ));
        assert!(matches!(
            keymap.map(key('r'), InputMode::Normal),
            Action::Edit
        ));
        assert!(matches!(
            keymap.map(key('R'), InputMode::Normal),
            Action::CycleItemType
        ));
    }
}
