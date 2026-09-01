use super::{app_error, move_index};
use crate::{
    app::{App, AppResult, Editor, FavoriteLinkForm, InputMode, LinkBankState, Popup},
    external,
    model::FavoriteLinkId,
};

impl App {
    pub(super) fn open_link_bank(&mut self) -> AppResult<()> {
        let (form, target) = match self.state.popup.take() {
            Some(Popup::Editor(Editor::Event { form, target })) => (form, target),
            popup => {
                self.state.popup = popup;
                return Err(app_error(
                    "favorite links are available in the event editor",
                ));
            }
        };
        self.state.link_bank = Some(LinkBankState {
            event_form: form,
            event_target: target,
            query: String::new(),
            items: Vec::new(),
            selected: 0,
            searching: false,
        });
        self.state.popup = Some(Popup::LinkBank);
        self.refresh_link_bank()?;
        self.sync_input_mode();
        Ok(())
    }

    pub(super) fn close_link_bank(&mut self) {
        if let Some(bank) = self.state.link_bank.take() {
            self.state.popup = Some(Popup::Editor(Editor::Event {
                form: bank.event_form,
                target: bank.event_target,
            }));
        } else {
            self.state.popup = None;
        }
        self.sync_input_mode();
    }

    pub(super) fn refresh_link_bank(&mut self) -> AppResult<()> {
        let query = self
            .state
            .link_bank
            .as_ref()
            .map(|bank| bank.query.clone())
            .unwrap_or_default();
        let items = self.database.search_favorite_links(&query)?;
        if let Some(bank) = self.state.link_bank.as_mut() {
            bank.items = items;
            bank.selected = bank.selected.min(bank.items.len().saturating_sub(1));
        }
        Ok(())
    }

    pub(super) fn move_link_bank(&mut self, delta: i32) {
        if let Some(bank) = self.state.link_bank.as_mut() {
            bank.selected = move_index(bank.selected, bank.items.len(), delta);
        }
    }

    pub(super) fn start_link_search(&mut self) {
        if let Some(bank) = self.state.link_bank.as_mut() {
            bank.searching = true;
            self.state.input_mode = InputMode::LinkSearch;
        }
    }

    pub(super) fn finish_link_search(&mut self) {
        if let Some(bank) = self.state.link_bank.as_mut() {
            bank.searching = false;
        }
        self.sync_input_mode();
    }

    pub(super) fn input_link_search(&mut self, character: char) -> AppResult<()> {
        if let Some(bank) = self.state.link_bank.as_mut() {
            bank.query.push(character);
        }
        self.refresh_link_bank()
    }

    pub(super) fn backspace_link_search(&mut self) -> AppResult<()> {
        if let Some(bank) = self.state.link_bank.as_mut() {
            _ = bank.query.pop();
        }
        self.refresh_link_bank()
    }

    pub(super) fn toggle_favorite_link(&mut self) -> AppResult<()> {
        let Some(link_id) = self
            .state
            .link_bank
            .as_ref()
            .and_then(|bank| bank.items.get(bank.selected).map(|link| link.id))
        else {
            return Ok(());
        };
        let bank = self
            .state
            .link_bank
            .as_mut()
            .ok_or_else(|| app_error("link bank is not open"))?;
        if let Some(index) = bank
            .event_form
            .favorite_link_ids
            .iter()
            .position(|id| *id == link_id)
        {
            bank.event_form.favorite_link_ids.remove(index);
        } else {
            bank.event_form.favorite_link_ids.push(link_id);
        }
        self.refresh_event_link_summary()
    }

    pub(super) fn add_favorite_link(&mut self) {
        if self.state.link_bank.is_some() {
            self.state.popup = Some(Popup::Editor(Editor::FavoriteLink {
                form: FavoriteLinkForm::default(),
                target: None,
            }));
            self.sync_input_mode();
        }
    }

    pub(super) fn edit_favorite_link(&mut self) {
        let selected = self
            .state
            .link_bank
            .as_ref()
            .and_then(|bank| bank.items.get(bank.selected).cloned());
        if let Some(link) = selected {
            self.state.popup = Some(Popup::Editor(Editor::FavoriteLink {
                form: FavoriteLinkForm::from_link(&link),
                target: Some(link.id),
            }));
            self.sync_input_mode();
        }
    }

    pub(super) fn finish_favorite_link(
        &mut self,
        form: FavoriteLinkForm,
        target: Option<FavoriteLinkId>,
    ) -> AppResult<()> {
        let link = form.values();
        let id = if let Some(id) = target {
            self.database.update_favorite_link(id, &link)?;
            id
        } else {
            self.database.create_favorite_link(&link)?
        };
        if target.is_none()
            && let Some(bank) = self.state.link_bank.as_mut()
            && !bank.event_form.favorite_link_ids.contains(&id)
        {
            bank.event_form.favorite_link_ids.push(id);
        }
        self.state.popup = Some(Popup::LinkBank);
        self.refresh_event_link_summary()?;
        self.refresh_link_bank()?;
        self.state.status_message = Some("Избранная ссылка сохранена".into());
        self.sync_input_mode();
        Ok(())
    }

    pub(super) fn open_favorite_link(&mut self) -> AppResult<()> {
        let url = self
            .state
            .link_bank
            .as_ref()
            .and_then(|bank| bank.items.get(bank.selected))
            .map(|link| link.url.clone())
            .ok_or_else(|| app_error("no favorite link selected"))?;
        external::open_url(&url)?;
        self.state.status_message = Some("Ссылка открыта".into());
        Ok(())
    }

    fn refresh_event_link_summary(&mut self) -> AppResult<()> {
        let ids = self
            .state
            .link_bank
            .as_ref()
            .map(|bank| bank.event_form.favorite_link_ids.clone())
            .unwrap_or_default();
        let links = self.database.favorite_links_by_ids(&ids)?;
        if let Some(bank) = self.state.link_bank.as_mut() {
            bank.event_form.set_favorite_links(&links);
        }
        Ok(())
    }
}
