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
            InputMode::GotoDate => self.goto_date(key),
            InputMode::Confirm => match key.code {
                KeyCode::Enter | KeyCode::Char('y' | 'Y' | 'д' | 'Д') => Action::Confirm(true),
                KeyCode::Esc | KeyCode::Char('n' | 'N' | 'н' | 'Н') => Action::Confirm(false),
                _ => Action::Noop,
            },
            InputMode::Scope => match key.code {
                KeyCode::Char('o' | 'O' | 'т' | 'Т') => Action::ChooseOccurrence,
                KeyCode::Char('s' | 'S' | 'с' | 'С') => Action::ChooseSeries,
                KeyCode::Esc => Action::Back,
                _ => Action::Noop,
            },
            InputMode::Normal => self.normal(key),
        }
    }

    fn editor(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => Action::Back,
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Submit,
            KeyCode::Enter | KeyCode::Tab | KeyCode::Down => Action::NextField,
            KeyCode::BackTab | KeyCode::Up => Action::PreviousField,
            KeyCode::Left => Action::AdjustLeft,
            KeyCode::Right => Action::AdjustRight,
            KeyCode::Backspace => Action::Backspace,
            KeyCode::Char(character) => Action::Input(character),
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

    fn normal(&mut self, key: KeyEvent) -> Action {
        if self.pending_g {
            self.pending_g = false;
            return match key.code {
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
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('h') | KeyCode::Left => Action::MoveLeft,
            KeyCode::Char('l') | KeyCode::Right => Action::MoveRight,
            KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
            KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
            KeyCode::Enter => Action::Open,
            KeyCode::Esc => Action::Back,
            KeyCode::Char('n') => Action::Create,
            KeyCode::Char('e') => Action::Edit,
            KeyCode::Char('d') => Action::Delete,
            KeyCode::Char('a') => Action::OpenAgenda,
            KeyCode::Char('t') => Action::OpenUpcoming,
            KeyCode::Char('p') => Action::ChangeImportance,
            KeyCode::Char('w') => Action::SwitchView(View::Week),
            KeyCode::Char('D') => Action::SwitchView(View::Day),
            KeyCode::Char('m') => Action::SwitchView(View::Month),
            KeyCode::Char('Y') => Action::SwitchView(View::Year),
            KeyCode::Char('?') => Action::Help,
            KeyCode::Char('/') => Action::StartSearch,
            KeyCode::Tab => Action::ToggleFocus,
            KeyCode::Char('f') => Action::CycleDateFilter,
            KeyCode::Char('r') => Action::CycleItemType,
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
            keymap.map(key('d'), InputMode::Normal),
            Action::StartGotoDate
        ));
    }
}
