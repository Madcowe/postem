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
use autonomi::{Bytes, Chunk, GraphEntry, PublicKey, SecretKey};

use crate::client::PostemClient;
use crate::error::PostemError;

/// A package to be deilvered consiting of it address which will be derived from the addressee
/// the payload arbitary data in Bytes and the seal the encrypted hex of the datamap of the
/// payload
pub struct Package {
    address: GraphEntry,
    seal: Chunk, //(Contains encrypted hex of data map of payolad),
    payload: Bytes,
}
impl PostemClient {
    pub async fn create(
        &self,
        location: &SecretKey,
        public_key: &PublicKey,
        payload: Bytes,
        payment_option: PaymentOption,
    ) -> Result<Package, PostemError> {
        let (data_cost, data_map) = self
            .client
            .data_put(payload.clone(), payment_option.clone())
            .await?;
        let seal = Chunk::new(Bytes::from(
            public_key.encrypt(data_map.to_hex()).to_bytes(),
        ));
        let (data_map_cost, addr) = self.client.chunk_put(&seal, payment_option.clone()).await?;
        let address = GraphEntry::new(
            &location,
            vec![public_key.clone()],
            [0u8; 32],
            vec![(public_key.clone(), addr.xorname().0)],
        );
        Ok(Package {
            address,
            seal,
            payload,
        })
    }
}
