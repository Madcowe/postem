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
    fn new() -> InputType {
        let k = KeyEvent::new_with_kind(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
            KeyEventKind::Press,
        );

        InputType::KeyPress(k)
    }

    fn create_key_press(key: KeyCode, key_modifers: KeyModifiers) -> InputType {
        InputType::KeyPress(KeyEvent::new_with_kind(
            key,
            key_modifers,
            KeyEventKind::Press,
        ))
    }
}

pub struct Action<T>
where
    T: Fn(App) -> (),
{
    name: String,
    description: String,
    closure: T,
}

// Doesn't need to be 2d as can just be hash of each AppState and InputType
// Think will need to used boxed trait object or maybe trait object reference to account for the
// different types of the closures
// https://stackoverflow.com/questions/27831944/how-do-i-store-a-closure-in-a-struct-in-rust
pub struct AppInteractions<T: Fn(App)> {
    // pub app: App,
    pub interactions: HashMap<AppState, HashMap<InputType, Action<T>>>,
}
impl<T: Fn(App)> AppInteractions<T> {
    pub fn new(&mut self) -> AppInteractions<T> {
        // From AppState::None
        let app_state = AppState::None;
        let input = InputType::create_key_press(KeyCode::Char('p'), KeyModifiers::empty());
        let closure =
            |app: App| app.change_state(AppState::PostPackagee(PostPackageState::InputRecipients));
        let action = Action {
            name: "Post Package".to_string(),
            description: "Press p to post a package".to_string(),
            closure,
        };
        let actions = HashMap::new();
        actions.insert(input, action);
        let interactions = HashMap::new();
        interactions.insert(app_state, actions);
        AppInteractions { interactions }
    }
}
