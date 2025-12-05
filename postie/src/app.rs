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
use autonomi::AttoTokens;
use autonomi::client::payment::PaymentOption;
use postem::{
    Addressee, ConnectionType, DoorMat, Package, PostemClient, PostemError, addressee::PostemName,
};

enum AppState {
    None,
    CreateAddresseei(CreateAddresseeState),
    SendPackagee(SendPackageStatus),
    ViewDoormat,
    ViewPackage,
}

enum CreateAddresseeState {
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

enum SendPackageStatus {
    InputRecipients,
    InputMessage,
    InputFundingWallet,
}
impl SendPackageStatus {
    pub fn toggle(&mut self, fowards: bool) {
        match self {
            SendPackageStatus::InputRecipients => {
                *self = if fowards {
                    SendPackageStatus::InputMessage
                } else {
                    SendPackageStatus::InputFundingWallet
                }
            }
            SendPackageStatus::InputMessage => {
                *self = if fowards {
                    SendPackageStatus::InputFundingWallet
                } else {
                    SendPackageStatus::InputRecipients
                }
            }
            SendPackageStatus::InputFundingWallet => {
                *self = if fowards {
                    SendPackageStatus::InputRecipients
                } else {
                    SendPackageStatus::InputMessage
                }
            }
        }
    }
}

struct App {
    app_state: AppState,
    client: PostemClient,
    door_mat: Option<DoorMat>,
    recipients: Vec<PostemName>,
    package: Option<Package>,
}
impl App {
    pub async fn create(connection_type: ConnectionType) -> Result<App, PostemError> {
        let client = PostemClient::init(connection_type).await?;
        Ok(App {
            app_state: AppState::None,
            client,
            door_mat: None,
            recipients: vec![],
            package: None,
        })
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

    /// Check for and receives any new items
    pub async fn update_doormat(&mut self) -> Result<bool, PostemError> {
        if let Some(ref mut door_mat) = self.door_mat {
            return Ok(self.client.doormat_update(door_mat).await?);
        } else {
            return Ok(false); // returns false if called when there is no doormat
        }
    }

    /// Returns a vector of all existing recipients
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

    // pub async fn estimate_postage(&self, pacakge: Package, no_of_recipients: usize) -> Result<AttoTokens, PostemError> {

    // }
}
