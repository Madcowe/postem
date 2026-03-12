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
use autonomi::client::payment::PaymentOption;
use autonomi::{AttoTokens, Bytes, SecretKey};
use postem::door_mat;
use postem::{
    Addressee, ConnectionType, DoorMat, Package, PostemClient, PostemError, addressee::PostemName,
};
use ratatui::crossterm::style::Stylize;

use crate::accounts::{Account, Accounts};
use crate::theme::Theme;
use crate::ui::wait_pop_up;

const ACCOUNTS_FILE_NAME: &str = "I_SHOULD_BE_ENCRYPTED.toml";

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AppState {
    About,
    None,
    Error,
    Confirm,
    Quit,
    CreateAddressee(CreateAddresseeState),
    ChooseAddressee,
    PostPackage(PostPackageState),
    ViewDoormat,
    ViewPackage,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CreateAddresseeState {
    InputAddresseeName,
    InputFundingWallet,
}
impl CreateAddresseeState {
    pub fn toggle(&self) -> CreateAddresseeState {
        match self {
            CreateAddresseeState::InputAddresseeName => CreateAddresseeState::InputFundingWallet,
            CreateAddresseeState::InputFundingWallet => CreateAddresseeState::InputAddresseeName,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PostPackageState {
    InputFundingWallet,
    InputRecipients,
    InputMessage,
}
impl PostPackageState {
    pub fn toggle(&self, fowards: bool) -> PostPackageState {
        match self {
            PostPackageState::InputFundingWallet => {
                if fowards {
                    PostPackageState::InputRecipients
                } else {
                    PostPackageState::InputMessage
                }
            }
            PostPackageState::InputRecipients => {
                if fowards {
                    PostPackageState::InputMessage
                } else {
                    PostPackageState::InputFundingWallet
                }
            }
            PostPackageState::InputMessage => {
                if fowards {
                    PostPackageState::InputFundingWallet
                } else {
                    PostPackageState::InputRecipients
                }
            }
        }
    }
}

pub struct App {
    connection_type: ConnectionType,
    app_state: AppState,
    pub status: String,
    previous_state: AppState,
    menu_visible: bool,
    client: PostemClient,
    error_text: Option<String>,
    pub(crate) theme: Theme,
    door_mat: Option<DoorMat>,
    door_mat_index: usize,
    accounts: Accounts,
    accounts_index: usize,
    char_input_buffer: Option<char>,
    post_recipients_input: String,
    post_message_input: String,
    post_key_input: String,
    create_name_input: String,
    create_key_input: String,
    cost_estimate: AttoTokens,
    vertical_scroll: u16,
}
impl App {
    pub async fn create(connection_type: ConnectionType) -> Result<App, PostemError> {
        let client = PostemClient::init(connection_type).await?;
        Ok(App {
            connection_type,
            app_state: AppState::None,
            status: String::new(),
            previous_state: AppState::None,
            menu_visible: false,
            client,
            error_text: None,
            theme: Theme::surf_bored_synth_wave(),
            door_mat: None,
            door_mat_index: 0,
            accounts: Accounts::new(),
            accounts_index: 0,
            char_input_buffer: None,
            post_recipients_input: String::new(),
            post_message_input: String::new(),
            post_key_input: String::new(),
            create_name_input: String::new(),
            create_key_input: String::new(),
            cost_estimate: AttoTokens::zero(),
            vertical_scroll: 0,
        })
    }

    pub fn change_state(&mut self, app_state: AppState) {
        match self.app_state {
            AppState::Error | AppState::Confirm => (),
            _ => self.previous_state = self.app_state,
        }
        if self.app_state == AppState::ViewPackage {
            self.vertical_scroll_reset();
        }
        self.app_state = app_state;
    }

    pub fn return_to_previous_state(&mut self) {
        self.app_state = self.previous_state;
    }

    pub fn app_state(&self) -> AppState {
        self.app_state.clone()
    }

    pub fn previous_state(&self) -> AppState {
        self.previous_state.clone()
    }

    pub fn theme(&self) -> Theme {
        self.theme.clone()
    }

    pub fn toggle_menu(&mut self) {
        self.menu_visible = match self.menu_visible {
            true => false,
            false => true,
        }
    }

    pub fn has_door_mat(&self) -> bool {
        self.door_mat.is_some()
    }

    pub fn door_mat_size(&self) -> usize {
        if let Some(door_mat) = &self.door_mat {
            door_mat.items().len()
        } else {
            0
        }
    }

    pub fn name_of_door_mat(&self) -> Option<String> {
        self.door_mat
            .clone()
            .map(|d| d.addressee().address().name().clone())
    }

    pub fn door_mat_as_table(&self) -> Vec<[String; 2]> {
        if !self.has_door_mat() {
            return vec![];
        }
        let mut v = vec![];
        for item in self.door_mat.clone().unwrap().items() {
            if let Some(text) = item.payload_as_string() {
                let lines: Vec<&str> = text.lines().collect();
                if lines.len() >= 2 {
                    v.push([lines[0].to_string(), lines[1].to_string()])
                } else if lines.len() == 1 {
                    v.push(["".to_string(), lines[0].to_string()])
                }
            }
        }
        v
    }

    pub fn get_current_package(&mut self) -> Option<String> {
        if self.app_state == AppState::ViewPackage && self.has_door_mat() {
            self.status = format!(
                "get_current_package: State: {:?} has doormat: {}",
                self.app_state,
                self.has_door_mat()
            );
            if let Some(item) = self
                .door_mat
                .clone()
                .unwrap()
                .items()
                .get(self.get_selected_item())
            {
                item.payload_as_string()
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn display_message(&mut self) -> (String, String) {
        let message = self.get_current_package().clone().unwrap_or(String::new());
        let mut lines: Vec<&str> = message.lines().collect();
        // let (sender, message) =
        if lines.len() >= 1 {
            (lines.remove(0).to_string(), lines.join("\n").to_string())
        } else {
            (String::new(), String::new())
        }
    }

    pub fn menu_visible(&self) -> bool {
        self.menu_visible
    }

    pub fn post_recipients_input(&self) -> &str {
        &self.post_recipients_input
    }

    pub fn post_message_bytes(&self) -> Bytes {
        // if message less that 3 characters add white space up to that as needs to be a lests 3 bytes
        let mut message = if self.has_door_mat() {
            self.door_mat.clone().unwrap().addressee().address().name()
        } else {
            "Anonymous".to_string()
        } + "\n"
            + &self.post_message_input.clone();
        if message.len() < 3 {
            let extra_blanks = 3 - message.len();
            for _ in 0..extra_blanks {
                message.push(' ');
            }
            message.push('.');
        }
        Bytes::from(message)
    }

    pub fn post_message_input(&self) -> &str {
        &self.post_message_input
    }

    pub fn post_key_input(&self) -> &str {
        &self.post_key_input
    }

    pub fn create_name_input(&self) -> String {
        self.create_name_input.to_string()
    }

    pub fn create_key_input(&self) -> String {
        self.create_key_input.to_string()
    }

    pub fn set_cost_estimate(&mut self, value: AttoTokens) {
        self.cost_estimate = value;
    }

    pub fn confirm_message(&self) -> Option<String> {
        if self.app_state == AppState::Confirm {
            match self.previous_state {
                AppState::CreateAddressee(CreateAddresseeState::InputFundingWallet) => {
                    Some(format!(
                        "Estimated cost of creating new address: {} attos.",
                        self.cost_estimate
                    ))
                }
                AppState::PostPackage(PostPackageState::InputMessage) => Some(format!(
                    "Estimate cost of posting package(s): {} attos.",
                    self.cost_estimate
                )),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn toggle_sub_state(&mut self, forward: bool) {
        match self.app_state {
            AppState::PostPackage(post_package_state) => {
                self.change_state(AppState::PostPackage(post_package_state.toggle(forward)));
            }
            AppState::CreateAddressee(create_addressee_state) => {
                self.change_state(AppState::CreateAddressee(create_addressee_state.toggle()));
            }
            _ => (),
        }
    }

    pub fn error_text(&self) -> Option<String> {
        self.error_text.clone()
    }

    pub fn set_error_text(&mut self, error_text: &str) {
        self.error_text = Some(error_text.to_string());
        self.change_state(AppState::Error);
    }

    pub fn clear_error_text(&mut self) {
        self.error_text = None;
    }

    pub fn set_chat_input_buffer(&mut self, value: char) {
        self.char_input_buffer = Some(value);
    }

    pub fn text_input(&mut self) {
        if let Some(char) = self.char_input_buffer {
            match self.app_state {
                AppState::PostPackage(PostPackageState::InputRecipients) => {
                    self.post_recipients_input.push(char);
                }
                AppState::PostPackage(PostPackageState::InputMessage) => {
                    self.post_message_input.push(char);
                }
                AppState::PostPackage(PostPackageState::InputFundingWallet) => {
                    self.post_key_input.push(char);
                }
                AppState::CreateAddressee(CreateAddresseeState::InputAddresseeName) => {
                    self.create_name_input.push(char);
                }
                AppState::CreateAddressee(CreateAddresseeState::InputFundingWallet) => {
                    self.create_key_input.push(char);
                }
                _ => (),
            }
        }
    }

    pub fn message_new_line(&mut self) {
        match self.app_state {
            AppState::PostPackage(PostPackageState::InputMessage) => {
                self.post_message_input += "\n";
            }
            _ => (),
        }
    }

    pub fn text_delete(&mut self) {
        match self.app_state {
            AppState::PostPackage(PostPackageState::InputRecipients) => {
                self.post_recipients_input.pop();
            }
            AppState::PostPackage(PostPackageState::InputMessage) => {
                self.post_message_input.pop();
            }
            AppState::PostPackage(PostPackageState::InputFundingWallet) => {
                self.post_key_input.pop();
            }
            AppState::CreateAddressee(CreateAddresseeState::InputAddresseeName) => {
                self.create_name_input.pop();
            }
            AppState::CreateAddressee(CreateAddresseeState::InputFundingWallet) => {
                self.create_key_input.pop();
            }
            _ => (),
        };
    }

    pub fn text_clear(&mut self) {
        match self.app_state {
            AppState::PostPackage(PostPackageState::InputRecipients) => {
                self.post_recipients_input = String::new();
            }
            AppState::PostPackage(PostPackageState::InputMessage) => {
                self.post_message_input = String::new();
            }
            AppState::PostPackage(PostPackageState::InputFundingWallet) => {
                self.post_key_input = String::new();
            }
            AppState::CreateAddressee(CreateAddresseeState::InputAddresseeName) => {
                self.create_name_input = String::new();
            }
            AppState::CreateAddressee(CreateAddresseeState::InputFundingWallet) => {
                self.create_key_input = String::new();
            }
            _ => (),
        }
    }

    pub fn next_item(&mut self) {
        match self.app_state {
            AppState::ChooseAddressee => {
                let total = self.accounts.size();
                if total > 0 && self.accounts_index < total - 1 {
                    self.accounts_index += 1
                } else if total > 0 && self.accounts_index >= total - 1 {
                    self.accounts_index = 0
                }
            }
            AppState::ViewDoormat => {
                let total = self.door_mat_size();
                if total > 0 && self.door_mat_index < total - 1 {
                    self.door_mat_index += 1
                } else if total > 0 && self.door_mat_index >= total - 1 {
                    self.door_mat_index = 0
                }
            }
            _ => (),
        }
    }

    pub fn previous_item(&mut self) {
        match self.app_state {
            AppState::ChooseAddressee => {
                let total = self.accounts.size();
                if total > 0 && self.accounts_index > 0 {
                    self.accounts_index -= 1
                } else if total > 0 && self.accounts_index == 0 {
                    self.accounts_index = total - 1
                }
            }
            AppState::ViewDoormat => {
                let total = self.door_mat_size();
                if total > 0 && self.door_mat_index > 0 {
                    self.door_mat_index -= 1
                } else if total > 0 && self.door_mat_index == 0 {
                    self.door_mat_index = total - 1
                }
            }
            _ => (),
        }
    }

    pub async fn select_item(&mut self) -> Result<(), PostemError> {
        match self.app_state {
            AppState::ChooseAddressee => {
                if let Some(account) = self.accounts.get_account(self.accounts_index) {
                    self.switch_addressee(account).await?;
                    self.change_state(AppState::ViewDoormat);
                }
            }
            AppState::ViewDoormat => {
                if self.door_mat_size() > 0 && self.door_mat_index < self.door_mat_size() {
                    self.change_state(AppState::ViewPackage);
                }
            }
            _ => (),
        }
        Ok(())
    }

    pub fn get_selected_item(&self) -> usize {
        match self.app_state() {
            AppState::ChooseAddressee => self.accounts_index,
            AppState::ViewDoormat | AppState::ViewPackage => self.door_mat_index,
            _ => 0,
        }
    }

    pub fn get_accounts_table(&self) -> Vec<String> {
        self.accounts.as_table()
    }

    /// If available returns estimated cost.
    pub async fn check_addressee_available(&self, name: &str) -> Result<AttoTokens, PostemError> {
        self.client
            .check_if_public_key_used(&PostemName::create(name)?.derive_key()?.public_key())
            .await?;
        self.client.addressee_cost(name).await
    }

    /// If succesful returns actual cost
    pub async fn create_addressee(
        &mut self,
        name: &str,
        private_key: &str,
    ) -> Result<AttoTokens, PostemError> {
        let (addressee, cost) = self
            .client
            .addressee_create(name, self.client.get_payment_option(private_key)?, None)
            .await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        self.accounts.add(addressee.clone().into());
        self.door_mat = Some(
            self.client
                .doormat_init(addressee.secret_key(), name)
                .await?,
        );
        let saved = self.save_accounts();
        self.status = format!("Saved: {:?}, accounts: {:?}", saved, self.accounts);
        if saved == false {
            // this will get immeditaly covered by another message so nedd to pass up
            let text = format!(
                "Failed to save addressee details to file, copy the following if you want to keep:\nName: {}\nSecret Key: {}",
                addressee.address().name(),
                addressee.secret_key().to_hex(),
            );
            self.set_error_text(&text);
        }
        self.change_state(AppState::None); // change to view dormat when implemented
        self.create_name_input = String::new();
        Ok(cost)
    }

    pub async fn switch_addressee(
        &mut self,
        account: Account,
        // name: &str,
        // private_key: &str,
    ) -> Result<(), PostemError> {
        let addressee = self
            .client
            .addressee_get(
                SecretKey::from_hex(&account.secret_key)?,
                PostemName::create(&account.name)?,
            )
            .await?;
        self.door_mat = Some(
            self.client
                .doormat_init(addressee.secret_key(), &account.name)
                .await?,
        );
        Ok(())
    }

    /// Check for and receives any new items
    pub async fn update_doormat(&mut self) -> Result<bool, PostemError> {
        if let Some(ref mut door_mat) = self.door_mat {
            return Ok(self.client.doormat_update(door_mat).await?);
        } else {
            return Ok(false); // returns false if called when there is no doormat
        }
    }

    /// Splits by ; or , and trims results
    pub fn split_recipients(&self) -> Vec<&str> {
        self.post_recipients_input
            .split(|c| c == ',' || c == ';')
            .map(|r| r.trim())
            .collect()
    }

    /// Returns a vector of all existing recipients, but only in terms of the key being used
    /// potentially you might want to check it is a valid postem address
    pub async fn check_recipients_exists<'a>(
        &self,
        names: Vec<&'a str>,
    ) -> Result<Vec<&'a str>, PostemError> {
        let mut existing_names = Vec::with_capacity(names.len());
        for name in names {
            if self
                .client
                .check_if_public_key_used(&PostemName::create(name)?.derive_key()?.public_key())
                .await?
            {
                existing_names.push(name);
            }
        }
        Ok(existing_names)
    }

    /// Return a vector of postem names for valid recipients plus a vector of any invalid names
    pub async fn check_recipients<'a>(
        &self,
        names: Vec<&'a str>,
    ) -> Result<(Vec<PostemName>, Vec<&'a str>), PostemError> {
        let valid_names = self.check_recipients_exists(names.clone()).await?;
        // let invalid_names = invalid_recipients(names, valid_names.clone());
        let invalid_names = subtract_vector(names, valid_names.clone());
        let mut postem_names = Vec::with_capacity(valid_names.len());
        for valid_name in valid_names {
            postem_names.push(PostemName::create(valid_name)?);
        }
        Ok((postem_names, invalid_names))
    }

    pub async fn estimate_postage(
        &self,
        payload: Bytes,
        no_of_recipients: usize,
    ) -> Result<AttoTokens, PostemError> {
        self.client.package_cost(payload, no_of_recipients).await
    }

    /// returns a vector of any recipients where posting failed plus the total cost
    pub async fn post_packages(
        &self,
        recipients: Vec<PostemName>,
        payload: Bytes,
        private_key: &str,
    ) -> Result<(Vec<PostemName>, AttoTokens), PostemError> {
        let mut cost = AttoTokens::zero();
        let mut failed_recipients = Vec::with_capacity(recipients.len());
        let payment_option = self.client.get_payment_option(private_key)?;
        // this isn't returning failed recipients properly
        for recipient in recipients {
            match self
                .client
                .package_post(recipient.clone(), payload.clone(), payment_option.clone())
                .await
            {
                Ok((_, package_cost)) => cost = cost.checked_add(package_cost).unwrap_or(cost),
                Err(_) => failed_recipients.push(recipient),
            }
        }
        Ok((failed_recipients, cost))
    }

    /// Returns true if succesfully loaded
    pub async fn load_accounts(&mut self) -> bool {
        match Accounts::load_file(ACCOUNTS_FILE_NAME) {
            Some(accounts) => {
                self.accounts = accounts;
                if self.accounts.size() > 0 {
                    self.change_state(AppState::ChooseAddressee);
                    // match self
                    //     .switch_addressee(self.accounts.get_account(0).unwrap())
                    //     .await
                    // {
                    //     Ok(_) => return true,
                    //     Err(_) => return false,
                    // }
                }
                return true;
            }
            None => return false,
        }
    }

    /// Returns true is succesfully saved
    pub fn save_accounts(&self) -> bool {
        self.accounts.save_file(ACCOUNTS_FILE_NAME)
    }

    pub fn accounts_size(&self) -> usize {
        self.accounts.size()
    }

    pub fn vertical_scroll(&self) -> u16 {
        self.vertical_scroll
    }

    pub fn vertical_scroll_reset(&mut self) {
        self.vertical_scroll = 0;
    }

    pub fn scroll_down(&mut self) {
        if self.vertical_scroll < std::u16::MAX {
            self.vertical_scroll += 1;
        }
    }

    pub fn scroll_up(&mut self) {
        if self.vertical_scroll > 0 {
            self.vertical_scroll -= 1;
        }
    }
}

/// Returns a vector of the element in vector a that were not present in vector b
pub fn subtract_vector<'a, T: PartialEq + ?Sized>(a: Vec<&'a T>, b: Vec<&'a T>) -> Vec<&'a T> {
    let mut not_in_vector_b = Vec::with_capacity(a.len());
    for a_element in a {
        let mut element_present = false;
        for b_element in &b {
            if a_element == *b_element {
                element_present = true;
                break;
            }
        }
        if !element_present {
            not_in_vector_b.push(a_element);
        }
    }
    not_in_vector_b
}

pub fn invalid_recipients<'a>(names: Vec<&'a str>, valid_names: Vec<&'a str>) -> Vec<&'a str> {
    let mut invalid_names = Vec::with_capacity(names.len());
    for name in names {
        let mut name_is_valid = false;
        for valid_name in &valid_names {
            if name == *valid_name {
                name_is_valid = true;
                break;
            }
        }
        if !name_is_valid {
            invalid_names.push(name);
        }
    }
    invalid_names
}

#[cfg(test)]

mod tests {

    use crate::accounts::Account;
    use crate::app::invalid_recipients;

    use super::*;

    #[test]
    pub fn test_subtract_vector() {
        let names = vec!["Eliza", "Elphine", "Pippy"];
        let valid_names = vec!["Eliza", "Pippy"];
        let invalid_names = subtract_vector(names, valid_names);
        assert_eq!(invalid_names, vec!["Elphine"]);
    }

    #[test]
    pub fn test_invalid_recipients() {
        let names = vec!["Eliza", "Elphine", "Pippy"];
        let valid_names = vec!["Eliza", "Pippy"];
        let invalid_names = invalid_recipients(names, valid_names);
        assert_eq!(invalid_names, vec!["Elphine"]);
    }

    #[tokio::test]
    pub async fn check_recipients() {
        let client = PostemClient::init(ConnectionType::Local).await.unwrap();
        let payment_option = client.get_payment_option("").unwrap();
        let name = "Eliza";
        let _addressee = client
            .addressee_create(name, payment_option, None)
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let app = App::create(ConnectionType::Local).await.unwrap();
        let (valid_names, invalid_names) = app
            .check_recipients(vec!["Eliza", "Elphine", "Pippy"])
            .await
            .unwrap();
        assert_eq!(valid_names, vec![PostemName::create(name).unwrap()]);
        assert_eq!(invalid_names, vec!["Elphine", "Pippy"]);
    }

    #[tokio::test]
    pub async fn estimate_and_post_packages() {
        let mut app = App::create(ConnectionType::Local).await.unwrap();
        // let client = PostemClient::init(ConnectionType::Local).await.unwrap();
        let payment_option = app.client.get_payment_option("").unwrap();
        let mut recipients = vec![];
        for _ in 0..2 {
            recipients.push(
                app.client
                    .addressee_create(&SecretKey::random().to_hex(), payment_option.clone(), None)
                    .await
                    .unwrap()
                    .0,
            );
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        // Note there is a minumum size otherwise self encryption fails...in text at least three character
        // let message = Bytes::from("AB");
        let message = Bytes::from("Could I interest you in these fine leather jackets?");
        let estimate = app.estimate_postage(message.clone(), 2).await.unwrap();
        // add non existing address
        let name = PostemName::create("nobody").unwrap();
        let mut recipient_names: Vec<PostemName> = recipients.iter().map(|r| r.address()).collect();
        recipient_names.push(name.clone());
        let (failed_recipients, cost) = app
            .post_packages(recipient_names, message.clone(), "")
            .await
            .unwrap();
        eprintln!("Estimate: {} Actual: {}", estimate, cost);
        assert_eq!(failed_recipients, vec![name]);
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let mut packages = Vec::with_capacity(recipients.len());
        for recipient in recipients {
            app.switch_addressee(Account {
                name: recipient.address().name(),
                secret_key: recipient.secret_key().to_hex(),
            })
            .await
            .unwrap();
            packages.append(&mut app.door_mat.clone().unwrap().items());
        }
        for package in packages {
            eprintln!("{:?}", package.payload());
            assert_eq!(package.payload().unwrap(), message);
        }
    }

    #[test]
    pub fn create_addressee_toggle() {
        let mut create_addressee_state = CreateAddresseeState::InputAddresseeName;
        create_addressee_state = create_addressee_state.toggle();
        assert_eq!(
            create_addressee_state,
            CreateAddresseeState::InputFundingWallet
        );
        create_addressee_state = create_addressee_state.toggle();
        assert_eq!(
            create_addressee_state,
            CreateAddresseeState::InputAddresseeName
        );
    }

    #[test]
    pub fn post_package_toggle() {
        let mut post_package_state = PostPackageState::InputRecipients;
        post_package_state = post_package_state.toggle(true);
        assert_eq!(post_package_state, PostPackageState::InputMessage);
        post_package_state = post_package_state.toggle(true);
        assert_eq!(post_package_state, PostPackageState::InputFundingWallet);
        post_package_state = post_package_state.toggle(true);
        assert_eq!(post_package_state, PostPackageState::InputRecipients);
        post_package_state = post_package_state.toggle(false);
        assert_eq!(post_package_state, PostPackageState::InputFundingWallet);
        post_package_state = post_package_state.toggle(false);
        assert_eq!(post_package_state, PostPackageState::InputMessage);
    }

    #[tokio::test]
    pub async fn toggle_sub_state() {
        let mut app = App::create(ConnectionType::Local).await.unwrap();
        app.change_state(AppState::PostPackage(PostPackageState::InputRecipients));
        assert_eq!(
            app.app_state,
            AppState::PostPackage(PostPackageState::InputRecipients),
        );
        app.toggle_sub_state(true);
        assert_eq!(
            app.app_state,
            AppState::PostPackage(PostPackageState::InputMessage),
        );
        app.toggle_sub_state(false);
        assert_eq!(
            app.app_state,
            AppState::PostPackage(PostPackageState::InputRecipients),
        );
    }

    #[tokio::test]
    pub async fn post_packages_to_invalid_addresses() {
        let app = App::create(ConnectionType::Local).await.unwrap();
        let private_key = SecretKey::random().to_hex();
        let payload = Bytes::from("Hello");
        let recipients = vec![PostemName::create("Nowhere").unwrap()];
        let (failed_recipients, cost) = app
            .post_packages(recipients.clone(), payload, &private_key)
            .await
            .unwrap();
        assert_eq!(recipients, failed_recipients);
        assert_eq!(cost, AttoTokens::zero());
    }
}
