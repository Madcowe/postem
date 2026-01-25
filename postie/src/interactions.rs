/*
Copyright (C) 2025-2026 Postem

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
use autonomi::{Bytes, PaymentMode};
use postem::PostemError;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::collections::HashMap;
use std::pin::Pin;
use tokio::time::{Duration, sleep};

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

    pub fn derive(app: &mut App, app_state: AppState, key_event: KeyEvent) -> InputType {
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
pub enum ToExecute {
    SyncFunction(fn(&mut App) -> Result<(), PostemError>),
    EstimateNewAddress,
    EstimatePostage,
}

#[derive(Clone, Debug)]
pub struct Action {
    menu_text: Option<String>,
    help_text: Option<String>,
    // function: fn(&mut App) -> Result<(), PostemError>,
    to_execute: ToExecute,
}
impl Action {
    pub fn create(
        menu_text: Option<&str>,
        help_text: Option<&str>,
        // function: fn(&mut App) -> Result<(), PostemError>,
        to_execute: ToExecute,
    ) -> Self {
        Self {
            menu_text: menu_text.map(|s| s.to_string()),
            help_text: help_text.map(|s| s.to_string()),
            to_execute,
        }
    }

    // executes if sync function or return async function to be passed to wait pop up
    pub fn execute_or_async<'a>(
        &'a self,
        app: &'a mut App,
    ) -> Result<
        Option<(
            Pin<Box<dyn Future<Output = Result<(), PostemError>> + 'a>>,
            String,
        )>,
        PostemError,
    > {
        match self.to_execute {
            ToExecute::SyncFunction(sync_function) => sync_function(app)?,
            ToExecute::EstimatePostage => {
                return Ok(Some((
                    Box::pin(estimate_postage(app)),
                    "Estimating postage cost...".to_string(),
                )));
            }
            ToExecute::EstimateNewAddress => {
                return Ok(Some((
                    Box::pin(estimate_addressee(app)),
                    "Estimating cost of creating address...".to_string(),
                )));
            }
        }
        Ok(None)
    }
}

// ------------------------------------------------------------------------------------------------
// synx functions that can be added to actions need signiture (&mut App) -> Result<(), PostemError>
// ------------------------------------------------------------------------------------------------

fn about(app: &mut App) -> Result<(), PostemError> {
    app.change_state(AppState::About);
    Ok(())
}

fn quit(app: &mut App) -> Result<(), PostemError> {
    app.change_state(AppState::Quit);
    Ok(())
}

fn close_error_pop_up(app: &mut App) -> Result<(), PostemError> {
    app.return_to_previous_state();
    app.clear_error_text();
    Ok(())
}

fn post_package(app: &mut App) -> Result<(), PostemError> {
    app.change_state(AppState::PostPackage(PostPackageState::InputRecipients));
    Ok(())
}

fn create_addressee(app: &mut App) -> Result<(), PostemError> {
    app.change_state(AppState::CreateAddressee(
        CreateAddresseeState::InputAddresseeName,
    ));
    Ok(())
}

fn text_input(app: &mut App) -> Result<(), PostemError> {
    app.text_input();
    Ok(())
}

fn text_delete(app: &mut App) -> Result<(), PostemError> {
    app.text_delete();
    Ok(())
}

fn text_clear(app: &mut App) -> Result<(), PostemError> {
    app.text_clear();
    Ok(())
}

fn toggle_sub_state(app: &mut App) -> Result<(), PostemError> {
    app.toggle_sub_state(true);
    Ok(())
}

fn toggle_sub_state_backwards(app: &mut App) -> Result<(), PostemError> {
    app.toggle_sub_state(false);
    Ok(())
}

fn confirm_transaction(app: &mut App) -> Result<(), PostemError> {
    app.set_transaction_confirmed(true);
    app.return_to_previous_state();
    Ok(())
}

fn cancel_transaction(app: &mut App) -> Result<(), PostemError> {
    app.set_transaction_confirmed(false);
    app.return_to_previous_state();
    Ok(())
}

fn leave_text_input(app: &mut App) -> Result<(), PostemError> {
    if app.has_doormat() {
        app.change_state(AppState::ViewDoormat);
    } else {
        app.change_state(AppState::None);
    }
    Ok(())
}

// ------------------------------------------------------------------------------------------------

// ------------------------------------------------------------------------------------------------
// async functions are returned by execute_or_async so that the main loop can send to wait pop up
// ------------------------------------------------------------------------------------------------

async fn estimate_addressee(app: &mut App) -> Result<(), PostemError> {
    app.set_cost_estimate(
        app.check_addressee_available(&app.create_name_input())
            .await?,
    );
    app.change_state(AppState::Confirm);
    while app.app_state() == AppState::Confirm {
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
    }
    if app.transaction_confirmed()
        && app.app_state() == AppState::CreateAddressee(CreateAddresseeState::InputFundingWallet)
    {
        app.create_addressee(&app.create_name_input(), &app.create_key_input())
            .await?;
    }
    Ok(())
}

async fn estimate_postage(app: &mut App) -> Result<(), PostemError> {
    let mut message = app.post_message_input();
    // if message less that 3 characters add white space up to that as needs to be a lests 3 bytes
    // to post...will be overkill for non-ascii but no need to boil the ocean.
    if message.len() < 3 {
        let extra_blanks = 3 - message.len();
        for _ in 0..extra_blanks {
            message.push(' ');
        }
        message.push('.');
    }
    let payload = Bytes::from(message);
    let no_of_recipients = app.split_recipients().len();
    app.set_cost_estimate(app.estimate_postage(payload, no_of_recipients).await?);
    app.change_state(AppState::Confirm);

    Ok(())
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
            ToExecute::SyncFunction(post_package),
        );
        actions.insert(input, action);
        let input = InputType::create_key_press(KeyCode::Char('c'), KeyModifiers::empty());
        let action = Action::create(
            Some("C Create address"),
            Some("Press c to create an address"),
            ToExecute::SyncFunction(create_addressee),
        );
        actions.insert(input, action);
        interactions.insert(app_state, actions);

        // From AppState::Error
        let app_state = AppState::Error;
        let mut actions = HashMap::new();
        let input = InputType::create_key_press(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let action = Action::create(None, None, ToExecute::SyncFunction(quit));
        actions.insert(input, action);
        let input = InputType::create_key_press(KeyCode::Char('q'), KeyModifiers::empty());
        let action = Action::create(None, None, ToExecute::SyncFunction(quit));
        actions.insert(input, action);
        let input = InputType::create_key_press(KeyCode::Enter, KeyModifiers::empty());
        let action = Action::create(
            None,
            Some("Press enter to continue"),
            ToExecute::SyncFunction(close_error_pop_up),
        );
        actions.insert(input, action);
        interactions.insert(app_state, actions);

        // From AppState::Confirm
        let app_state = AppState::Confirm;
        let mut actions = HashMap::new();
        let input = InputType::create_key_press(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let action = Action::create(None, None, ToExecute::SyncFunction(quit));
        actions.insert(input, action);
        let input = InputType::create_key_press(KeyCode::Char('q'), KeyModifiers::empty());
        let action = Action::create(None, None, ToExecute::SyncFunction(quit));
        actions.insert(input, action);
        let input = InputType::create_key_press(KeyCode::Char('y'), KeyModifiers::empty());
        let action = Action::create(
            None,
            Some("Press y to confirm"),
            ToExecute::SyncFunction(close_error_pop_up),
        );
        actions.insert(input, action);
        interactions.insert(app_state, actions);

        // From AppState::CreateAddressee(CreateAddresseeState::InputAddresseeName)
        let app_state = AppState::CreateAddressee(CreateAddresseeState::InputAddresseeName);
        let mut actions = create_text_input_actions();
        let action = Action::create(None, None, ToExecute::SyncFunction(toggle_sub_state));
        let input = InputType::create_key_press(KeyCode::Enter, KeyModifiers::empty());
        actions.insert(input, action.clone());
        interactions.insert(app_state, actions);
        // From AppState::CreateAddressee(CreateAddresseeState::InputFundingWallet)
        let app_state = AppState::CreateAddressee(CreateAddresseeState::InputFundingWallet);
        let mut actions = create_text_input_actions();
        let action = Action::create(None, None, ToExecute::EstimatePostage);
        let input = InputType::create_key_press(KeyCode::Enter, KeyModifiers::empty());
        actions.insert(input, action);
        interactions.insert(app_state, actions);

        // From AppState::PostPackage(PostPackageState::InputRecipients)
        let app_state = AppState::PostPackage(PostPackageState::InputRecipients);
        let mut actions = create_text_input_actions();
        let action = Action::create(None, None, ToExecute::SyncFunction(toggle_sub_state));
        let input = InputType::create_key_press(KeyCode::Enter, KeyModifiers::empty());
        actions.insert(input, action.clone());
        interactions.insert(app_state, actions);
        // From AppState::PostPackage(PostPackageState::InputMessage)
        let app_state = AppState::PostPackage(PostPackageState::InputMessage);
        let mut actions = create_text_input_actions();
        let action = Action::create(None, None, ToExecute::SyncFunction(toggle_sub_state));
        let input = InputType::create_key_press(KeyCode::Enter, KeyModifiers::empty());
        actions.insert(input, action.clone());
        interactions.insert(app_state, actions);
        // From AppState::PostPackage(PostPackageState::InputFundingWallet)
        let app_state = AppState::PostPackage(PostPackageState::InputFundingWallet);
        let mut actions = create_text_input_actions();
        let action = Action::create(None, None, ToExecute::EstimatePostage);
        let input = InputType::create_key_press(KeyCode::Enter, KeyModifiers::empty());
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
    let action = Action::create(None, None, ToExecute::SyncFunction(quit));
    actions.insert(input, action);
    let input = InputType::create_key_press(KeyCode::Char('q'), KeyModifiers::empty());
    let action = Action::create(Some("Q Quit"), None, ToExecute::SyncFunction(quit));
    actions.insert(input, action);
    let input = InputType::create_key_press(KeyCode::Char('a'), KeyModifiers::empty());
    let action = Action::create(Some("A About"), None, ToExecute::SyncFunction(post_package));
    actions.insert(input, action);
    actions
}

fn create_text_input_actions() -> HashMap<InputType, Action> {
    let mut actions = HashMap::new();
    let input = InputType::create_key_press(KeyCode::Char('c'), KeyModifiers::CONTROL);
    let action = Action::create(None, None, ToExecute::SyncFunction(quit));
    actions.insert(input, action);
    let input = InputType::TextInput;
    let action = Action::create(None, None, ToExecute::SyncFunction(text_input));
    actions.insert(input, action);
    let input = InputType::create_key_press(KeyCode::Backspace, KeyModifiers::empty());
    let action = Action::create(None, None, ToExecute::SyncFunction(text_delete));
    actions.insert(input, action);
    let input = InputType::create_key_press(KeyCode::Char('u'), KeyModifiers::CONTROL);
    let action = Action::create(None, None, ToExecute::SyncFunction(text_clear));
    actions.insert(input, action);
    let input = InputType::create_key_press(KeyCode::Esc, KeyModifiers::empty());
    let action = Action::create(None, None, ToExecute::SyncFunction(leave_text_input));
    actions.insert(input, action);
    let input = InputType::create_key_press(KeyCode::Tab, KeyModifiers::empty());
    let action = Action::create(None, None, ToExecute::SyncFunction(toggle_sub_state));
    actions.insert(input, action);
    let action = Action::create(
        None,
        None,
        ToExecute::SyncFunction(toggle_sub_state_backwards),
    );
    // Does shift need to be spefified with BackTab??? needs testing in app
    let input = InputType::create_key_press(KeyCode::BackTab, KeyModifiers::empty());
    actions.insert(input, action);
    actions
}

#[cfg(test)]

mod tests {

    // use crate::ui::wait_pop_up;

    use super::*;
    // use ratatui::{
    //     Terminal,
    //     backend::{Backend, CrosstermBackend},
    //     buffer::Buffer,
    //     layout::Rect,
    // };
    // use std::io;

    #[tokio::test]
    async fn test_app_interactions() {
        let mut app = App::create(postem::ConnectionType::Local).await.unwrap();
        assert_eq!(app.app_state(), AppState::None);
        let interactions = AppInteractions::new();
        eprintln!("{:?}", interactions.get_actions(AppState::None));
        let input = InputType::create_key_press(KeyCode::Char('p'), KeyModifiers::empty());
        let action = interactions.get(AppState::None, input).unwrap();
        action.execute_or_async(&mut app).unwrap();
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
        action.execute_or_async(&mut app).unwrap();
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
        action.execute_or_async(&mut app).unwrap();
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
        action.execute_or_async(&mut app).unwrap();
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
        action.execute_or_async(&mut app).unwrap();
        assert_eq!(app.post_recipients_input(), "");
        let input = InputType::derive(
            &mut app,
            AppState::PostPackage(PostPackageState::InputRecipients),
            KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()),
        );
        let action = interactions
            .get(
                AppState::PostPackage(PostPackageState::InputRecipients),
                input,
            )
            .unwrap();
        action.execute_or_async(&mut app).unwrap();
        assert_eq!(
            app.app_state(),
            AppState::PostPackage(PostPackageState::InputMessage)
        );
        let input = InputType::derive(
            &mut app,
            AppState::PostPackage(PostPackageState::InputMessage),
            KeyEvent::new(KeyCode::Char('A'), KeyModifiers::empty()),
        );
        let action = interactions
            .get(AppState::PostPackage(PostPackageState::InputMessage), input)
            .unwrap();
        action.execute_or_async(&mut app).unwrap();
        assert_eq!(app.post_message_input(), "A");
        // test pressing enter of InputFundingWallet
        let input = InputType::derive(
            &mut app,
            AppState::PostPackage(PostPackageState::InputFundingWallet),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );
        let action = interactions
            .get(
                AppState::PostPackage(PostPackageState::InputFundingWallet),
                input,
            )
            .unwrap();
        // let theme = app.theme();
        app.change_state(AppState::PostPackage(PostPackageState::InputMessage));
        eprintln!("{}", app.post_message_input());
        let async_fn = action.execute_or_async(&mut app).unwrap();
        assert!(async_fn.is_some());
        // commented out as makes test out put hard to read
        // let mut stdout = io::stdout();
        // let backend = CrosstermBackend::new(stdout);
        // let mut terminal = Terminal::new(backend).unwrap();
        // let buffer = Buffer::empty(Rect::new(0, 0, 1, 1));
        // let wait_result = wait_pop_up(
        //     &mut terminal,
        //     buffer,
        //     async_fn.unwrap(),
        //     "Testing...1...2...3",
        //     theme,
        // )
        // .await;
        // assert_eq!(wait_result, Ok(()));
    }
}
