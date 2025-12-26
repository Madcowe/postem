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

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Interaction(AppState, InputType);

#[derive(Clone, Debug)]
pub struct Action {
    name: String,
    description: String,
    function: fn(&mut App),
}
impl Action {
    fn create(name: &str, description: &str, function: fn(&mut App)) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            function,
        }
    }
}

fn none_to_post(app: &mut App) {
    app.change_state(AppState::PostPackage(PostPackageState::InputRecipients));
}

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
        let action = Action::create("Post package", "Press p to post a package", none_to_post);
        actions.insert(input, action);
        interactions.insert(app_state, actions);
        // interactions.insert(Interaction(app_state, input), action);
        AppInteractions { interactions }
    }

    pub fn get(&self, app_state: AppState, input: InputType) -> Option<Action> {
        if let Some(actions) = self.interactions.get(&app_state) {
            if let Some(action) = actions.get(&input) {
                return Some(action.clone());
            }
        }
        None
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
        let input = InputType::create_key_press(KeyCode::Char('p'), KeyModifiers::empty());
        let action = interactions.get(AppState::None, input).unwrap();
        (action.function)(&mut app);
        assert_eq!(
            app.app_state(),
            AppState::PostPackage(PostPackageState::InputRecipients)
        );
    }
}
