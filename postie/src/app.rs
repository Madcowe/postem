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
use autonomi::client::payment::PaymentOption;
use autonomi::{AttoTokens, Bytes, SecretKey};
use postem::{
    Addressee, ConnectionType, DoorMat, Package, PostemClient, PostemError, addressee::PostemName,
};

use crate::theme::Theme;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AppState {
    About,
    None,
    Quit,
    CreateAddressee(CreateAddresseeState),
    PostPackage(PostPackageState),
    ViewDoormat,
    ViewPackage,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CreateAddresseeState {
    InputAddresseeName,
    InputFundingWalled,
}
impl CreateAddresseeState {
    pub fn toggle(&mut self) {
        match self {
            CreateAddresseeState::InputAddresseeName => {
                *self = CreateAddresseeState::InputFundingWalled
            }
            CreateAddresseeState::InputFundingWalled => {
                *self = CreateAddresseeState::InputAddresseeName
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PostPackageState {
    InputRecipients,
    InputMessage,
    InputFundingWallet,
}
impl PostPackageState {
    pub fn toggle(&mut self, fowards: bool) {
        match self {
            PostPackageState::InputRecipients => {
                *self = if fowards {
                    PostPackageState::InputMessage
                } else {
                    PostPackageState::InputFundingWallet
                }
            }
            PostPackageState::InputMessage => {
                *self = if fowards {
                    PostPackageState::InputFundingWallet
                } else {
                    PostPackageState::InputRecipients
                }
            }
            PostPackageState::InputFundingWallet => {
                *self = if fowards {
                    PostPackageState::InputRecipients
                } else {
                    PostPackageState::InputMessage
                }
            }
        }
    }
}

pub struct App {
    connection_type: ConnectionType,
    app_state: AppState,
    client: PostemClient,
    theme: Theme,
    door_mat: Option<DoorMat>,
    // recipients: Vec<PostemName>,
    // payload: Option<Bytes>,
    char_input_buffer: Option<char>,
    post_recipients_input: String,
    post_message_input: String,
    post_key_input: String,
    create_name_input: String,
    create_key_input: String,
}
impl App {
    pub fn change_state(&mut self, app_state: AppState) {
        self.app_state = app_state;
    }

    pub fn app_state(&self) -> AppState {
        self.app_state
    }

    pub async fn create(connection_type: ConnectionType) -> Result<App, PostemError> {
        let client = PostemClient::init(connection_type).await?;
        Ok(App {
            connection_type,
            app_state: AppState::None,
            client,
            theme: Theme::surf_bored_synth_wave(),
            door_mat: None,
            // recipients: vec![],
            // payload: None,
            char_input_buffer: None,
            post_recipients_input: String::new(),
            post_message_input: String::new(),
            post_key_input: String::new(),
            create_name_input: String::new(),
            create_key_input: String::new(),
        })
    }

    pub fn set_chat_input_buffer(&mut self, value: char) {
        self.char_input_buffer = Some(value);
    }

    pub fn text_input(&mut self) {
        if let Some(char) = self.char_input_buffer {
            match self.app_state {
                AppState::PostPackage(PostPackageState::InputRecipients) => {
                    self.post_recipients_input.push(char)
                }
                AppState::PostPackage(PostPackageState::InputMessage) => {
                    self.post_message_input.push(char)
                }
                AppState::PostPackage(PostPackageState::InputFundingWallet) => {
                    self.post_key_input.push(char)
                }
                AppState::CreateAddressee(CreateAddresseeState::InputAddresseeName) => {
                    self.create_name_input.push(char)
                }
                AppState::CreateAddressee(CreateAddresseeState::InputFundingWalled) => {
                    self.create_key_input.push(char)
                }
                _ => (),
            }
        }
    }

    pub fn text_delete(&mut self) {
        match self.app_state {
            AppState::PostPackage(PostPackageState::InputRecipients) => {
                self.post_recipients_input.pop()
            }
            AppState::PostPackage(PostPackageState::InputMessage) => self.post_message_input.pop(),
            AppState::PostPackage(PostPackageState::InputFundingWallet) => {
                self.post_key_input.pop()
            }
            AppState::CreateAddressee(CreateAddresseeState::InputAddresseeName) => {
                self.create_name_input.pop()
            }
            AppState::CreateAddressee(CreateAddresseeState::InputFundingWalled) => {
                self.create_key_input.pop()
            }
            _ => None,
        };
    }

    pub fn text_clear(&mut self) {
        match self.app_state {
            AppState::PostPackage(PostPackageState::InputRecipients) => {
                self.post_recipients_input = String::new()
            }
            AppState::PostPackage(PostPackageState::InputMessage) => {
                self.post_message_input = String::new()
            }
            AppState::PostPackage(PostPackageState::InputFundingWallet) => {
                self.post_key_input = String::new()
            }
            AppState::CreateAddressee(CreateAddresseeState::InputAddresseeName) => {
                self.create_name_input = String::new()
            }
            AppState::CreateAddressee(CreateAddresseeState::InputFundingWalled) => {
                self.create_key_input = String::new()
            }
            _ => (),
        }
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
        self.door_mat = Some(
            self.client
                .doormat_init(addressee.secret_key(), name)
                .await?,
        );
        Ok(cost)
    }

    pub async fn import_addressee(
        &mut self,
        name: &str,
        private_key: &str,
    ) -> Result<(), PostemError> {
        let addressee = self
            .client
            .addressee_get(SecretKey::from_hex(private_key)?, PostemName::create(name)?)
            .await?;
        self.door_mat = Some(
            self.client
                .doormat_init(addressee.secret_key(), name)
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
            app.import_addressee(
                &recipient.address().name(),
                &recipient.secret_key().to_hex(),
            )
            .await
            .unwrap();
            packages.append(&mut app.door_mat.clone().unwrap().items());
        }
        for package in packages {
            eprintln!("{:?}", package.payload());
            assert_eq!(package.payload().unwrap(), message);
        }
    }
}
