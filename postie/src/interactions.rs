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
    TextInput, // a genetic state so we don't have to write every possible charcter on the keyboard
}
impl InputType {
    fn create_key_press(key: KeyCode, key_modifers: KeyModifiers) -> InputType {
        InputType::KeyPress(KeyEvent::new_with_kind(
            key,
            key_modifers,
            KeyEventKind::Press,
        ))
    }

    fn derive(app: &mut App, app_state: AppState, key_event: KeyEvent) -> InputType {
        match app_state {
            AppState::CreateAddressee(_) | AppState::PostPackage(_) => {
                if key_event.modifiers.is_empty() || key_event.modifiers == KeyModifiers::SHIFT {
                    if let KeyCode::Char(char) = key_event.code {
                        app.set_chat_input_buffer(char);
                        return InputType::TextInput;
                    };
                }
            }
            _ => (),
        }
        InputType::KeyPress(key_event)
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
// functions that can be added to actions must have signiture (&mut App)
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

fn text_input(app: &mut App) {
    app.text_input();
}

fn text_delete(app: &mut App) {
    app.text_delete();
}

fn text_clear(app: &mut App) {
    app.text_clear();
}

fn toggle_sub_state(app: &mut App) {
    app.toggle_sub_state(true);
}

fn toggle_sub_state_backwards(app: &mut App) {
    app.toggle_sub_state(false);
}

// ------------------------------------------------------------------------------------------------

pub struct AppInteractions {
    pub interactions: HashMap<AppState, HashMap<InputType, Action>>,
}
impl AppInteractions {
    pub fn new() -> AppInteractions {
        let mut interactions = HashMap::new();
        // From AppState::None
        let app_state = AppState::None;
        let mut actions = create_standard_actions();
        let input = InputType::create_key_press(KeyCode::Char('p'), KeyModifiers::empty());
        let action = Action::create(
            Some("P Post package"),
            Some("Press p to post a package"),
            post_package,
        );
        actions.insert(input, action);
        let input = InputType::create_key_press(KeyCode::Char('c'), KeyModifiers::empty());
        let action = Action::create(
            Some("C Create address"),
            Some("Press c to create an address"),
            create_addressee,
        );
        actions.insert(input, action);
        interactions.insert(app_state, actions);

        // From AppState::PostPackage(PostPackageState::InputRecipients)
        let app_state = AppState::PostPackage(PostPackageState::InputRecipients);
        let mut actions = create_text_input_actions();
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
            return actions.values().collect();
        }
        Vec::new()
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

fn create_standard_actions() -> HashMap<InputType, Action> {
    let mut actions = HashMap::new();
    let input = InputType::create_key_press(KeyCode::Char('c'), KeyModifiers::CONTROL);
    let action = Action::create(None, None, quit);
    actions.insert(input, action);
    let input = InputType::create_key_press(KeyCode::Char('q'), KeyModifiers::empty());
    let action = Action::create(Some("Q Quit"), None, quit);
    actions.insert(input, action);
    let input = InputType::create_key_press(KeyCode::Char('a'), KeyModifiers::empty());
    let action = Action::create(Some("A About"), None, post_package);
    actions.insert(input, action);
    actions
}

fn create_text_input_actions() -> HashMap<InputType, Action> {
    let mut actions = HashMap::new();
    let input = InputType::create_key_press(KeyCode::Char('c'), KeyModifiers::CONTROL);
    let action = Action::create(None, None, quit);
    actions.insert(input, action);
    let input = InputType::TextInput;
    let action = Action::create(None, None, text_input);
    actions.insert(input, action);
    let input = InputType::create_key_press(KeyCode::Backspace, KeyModifiers::empty());
    let action = Action::create(None, None, text_delete);
    actions.insert(input, action);
    let input = InputType::create_key_press(KeyCode::Char('u'), KeyModifiers::CONTROL);
    let action = Action::create(None, None, text_clear);
    actions.insert(input, action);
    let action = Action::create(None, None, toggle_sub_state);
    let input = InputType::create_key_press(KeyCode::Enter, KeyModifiers::empty());
    actions.insert(input, action.clone());
    let input = InputType::create_key_press(KeyCode::Tab, KeyModifiers::empty());
    actions.insert(input, action);
    let action = Action::create(None, None, toggle_sub_state_backwards);
    // Does shift need to be spefified with BackTab??? needs testing in app
    let input = InputType::create_key_press(KeyCode::BackTab, KeyModifiers::empty());
    actions.insert(input, action);
    actions
}

// fn create_input_function()

// fn create_text_input_actions(app_state: AppState) -> HashMap<InputType, Action> {
//     let mut actions = HashMap::new();
//     match app_state {
//         AppState::CreateAddressee(CreateAddresseeState::InputAddresseeName) =>
//     }

// }

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
        eprintln!(
            "{:?}",
            interactions.get_actions(AppState::PostPackage(PostPackageState::InputRecipients))
        );
        let input = InputType::derive(
            &mut app,
            AppState::PostPackage(PostPackageState::InputRecipients),
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()),
        );
        let action = interactions
            .get(
                AppState::PostPackage(PostPackageState::InputRecipients),
                input,
            )
            .unwrap();
        (action.function)(&mut app);
        assert_eq!(app.post_recipients_input(), "a");
        let input = InputType::derive(
            &mut app,
            AppState::PostPackage(PostPackageState::InputRecipients),
            KeyEvent::new(KeyCode::Char('m'), KeyModifiers::empty()),
        );
        let action = interactions
            .get(
                AppState::PostPackage(PostPackageState::InputRecipients),
                input,
            )
            .unwrap();
        (action.function)(&mut app);
        assert_eq!(app.post_recipients_input(), "am");
        let input = InputType::derive(
            &mut app,
            AppState::PostPackage(PostPackageState::InputRecipients),
            KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty()),
        );
        let action = interactions
            .get(
                AppState::PostPackage(PostPackageState::InputRecipients),
                input,
            )
            .unwrap();
        (action.function)(&mut app);
        assert_eq!(app.post_recipients_input(), "a");
        let input = InputType::derive(
            &mut app,
            AppState::PostPackage(PostPackageState::InputRecipients),
            KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        );
        let action = interactions
            .get(
                AppState::PostPackage(PostPackageState::InputRecipients),
                input,
            )
            .unwrap();
        (action.function)(&mut app);
        assert_eq!(app.post_recipients_input(), "");
    }
}
