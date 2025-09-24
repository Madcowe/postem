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

use autonomi::Client;

use crate::error::PostemError;

#[derive(Clone, Copy)]
pub enum ConnectionType {
    Local,
    Antnet,
}

pub struct PostemClient {
    connection_type: ConnectionType,
    client: Client,
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
}
