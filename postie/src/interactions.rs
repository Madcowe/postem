/*
Copyright (C) 2025 Postem

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as
published by the Free Software Foundation, either version 3 of the
License, or (at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::collections::HashMap;

use crate::app::{App, AppState, CreateAddresseeState, PostPackageState};

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum InputType {
    KeyPress(KeyEvent),
}
impl InputType {
    fn create_key_press(key: KeyCode, key_modifers: KeyModifiers) -> InputType {
        InputType::KeyPress(KeyEvent::new_with_kind(
            key,
            key_modifers,
            KeyEventKind::Press,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct Action {
    menu_text: Option<String>,
    help_text: Option<String>,
    function: fn(&mut App),
}
impl Action {
    fn create(menu_text: Option<&str>, help_text: Option<&str>, function: fn(&mut App)) -> Self {
        Self {
            menu_text: menu_text.map(|s| s.to_string()),
            help_text: help_text.map(|s| s.to_string()),
            function,
        }
    }
}

// ------------------------------------------------------------------------------------------------
// functions that can be added to actions
// ------------------------------------------------------------------------------------------------
fn about(app: &mut App) {
    app.change_state(AppState::About);
}

fn quit(app: &mut App) {
    app.change_state(AppState::Quit);
}

fn post_package(app: &mut App) {
    app.change_state(AppState::PostPackage(PostPackageState::InputRecipients));
}

fn create_addressee(app: &mut App) {
    app.change_state(AppState::CreateAddressee(
        CreateAddresseeState::InputAddresseeName,
    ));
}

// ------------------------------------------------------------------------------------------------

// move back to using 2d hashmap as then can return all Inputs of each state
pub struct AppInteractions {
    pub interactions: HashMap<AppState, HashMap<InputType, Action>>,
}
impl AppInteractions {
    pub fn new() -> AppInteractions {
        let mut interactions = HashMap::new();
        // From AppState::None
        let mut actions = HashMap::new();
        let app_state = AppState::None;
        let input = InputType::create_key_press(KeyCode::Char('p'), KeyModifiers::empty());
        let action = Action::create(
            Some("P Post package"),
            Some("Press p to post a package"),
            post_package,
        );
        actions.insert(input, action);
        interactions.insert(app_state, actions);
        AppInteractions { interactions }
    }

    pub fn get(&self, app_state: AppState, input: InputType) -> Option<Action> {
        // check inputs that are irreapective of state
        if let Some(actions) = self.interactions.get(&app_state) {
            if let Some(action) = actions.get(&input) {
                return Some(action.clone());
            }
        }
        None
    }

    pub fn get_actions(&self, app_state: AppState) -> Vec<&Action> {
        if let Some(actions) = self.interactions.get(&app_state) {
            return Some(actions.values().collect());
        }
        None
    }

    pub fn get_menu_items(&self, app_state: AppState) -> Vec<String> {
        let mut menu_items = Vec::new();
        for action in self.get_actions(app_state) {
            if let Some(menu_item) = &action.menu_text {
                menu_items.push(menu_item.clone());
            }
        }
        menu_items
    }

    pub fn get_help_items(&self, app_state: AppState) -> Vec<String> {
        let mut help_items = Vec::new();
        for action in self.get_actions(app_state) {
            if let Some(help_item) = &action.help_text {
                help_items.push(help_item.clone());
            }
        }
        help_items
    }
}

#[cfg(test)]

mod tests {

    use super::*;

    #[tokio::test]
    async fn test_app_interactions() {
        let mut app = App::create(postem::ConnectionType::Local).await.unwrap();
        assert_eq!(app.app_state(), AppState::None);
        let interactions = AppInteractions::new();
        eprintln!("{:?}", interactions.get_actions(AppState::None));
        let input = InputType::create_key_press(KeyCode::Char('p'), KeyModifiers::empty());
        let action = interactions.get(AppState::None, input).unwrap();
        (action.function)(&mut app);
        assert_eq!(
            app.app_state(),
            AppState::PostPackage(PostPackageState::InputRecipients)
        );
    }
}
