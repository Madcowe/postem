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
use autonomi::{Client, Wallet};

use crate::error::PostemError;

#[derive(Clone, Copy)]
pub enum ConnectionType {
    Local,
    Antnet,
}

pub struct PostemClient {
    pub(crate) connection_type: ConnectionType,
    pub(crate) client: Client,
}
impl PostemClient {
    pub async fn init(connection_type: ConnectionType) -> Result<PostemClient, PostemError> {
        // let connection_type = connection_type;
        let client = match connection_type {
            ConnectionType::Antnet => match Client::init().await {
                Err(_) => return Err(PostemError::ClientConnectionError),
                Ok(client) => client,
            },

            ConnectionType::Local => match Client::init_local().await {
                Err(_) => return Err(PostemError::ClientConnectionError),
                Ok(client) => client,
            },
        };
        Ok(PostemClient {
            connection_type,
            client,
        })
    }

    pub fn get_payment_option(&self, private_key: &str) -> Result<PaymentOption, PostemError> {
        Ok(PaymentOption::from(self.get_funded_wallet(&private_key)?))
    }

    pub fn get_funded_wallet(&self, private_key: &str) -> Result<Wallet, PostemError> {
        let private_key = match self.connection_type {
            ConnectionType::Antnet => private_key,
            ConnectionType::Local => {
                "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
            }
        };
        let wallet =
            match Wallet::new_from_private_key(self.client.evm_network().clone(), private_key) {
                Ok(wallet) => wallet,
                Err(e) => {
                    return Err(PostemError::FailedToGetWallet(
                        private_key.to_string(),
                        format!("{:?}", e),
                    ));
                }
            };
        Ok(wallet)
    }
}
