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
use autonomi::{AttoTokens, Bytes, Chunk, GraphEntry, GraphEntryAddress, PublicKey, SecretKey};

use crate::client::PostemClient;
use crate::error::PostemError;

/// A package to be deilvered consiting of it address which will be derived from the addressee
/// the payload arbitary data in Bytes and the seal the encrypted hex of the datamap of the
/// payload
#[derive(Debug, PartialEq)]
pub struct Package {
    address: GraphEntry,
    seal: Chunk, // Contains encrypted hex of data map of payload
    payload: Bytes,
    cost: AttoTokens,
}
impl Package {
    pub fn address(&self) -> GraphEntry {
        self.address.clone()
    }
}
impl PostemClient {
    pub async fn package_create(
        &self,
        location: &SecretKey,
        public_key: &PublicKey,
        payload: Bytes,
        payment_option: PaymentOption,
    ) -> Result<Package, PostemError> {
        let (payload_cost, data_map) = self
            .client
            .data_put(payload.clone(), payment_option.clone())
            .await?;
        let seal = Chunk::new(Bytes::from(
            public_key.encrypt(data_map.to_hex()).to_bytes(),
        ));
        let (seal_cost, addr) = self.client.chunk_put(&seal, payment_option.clone()).await?;
        let address = GraphEntry::new(
            &location,
            vec![],
            location.to_bytes(), // location needs to be here so next one can be derived if jupmed to
            vec![(public_key.clone(), addr.xorname().0)],
        );
        let (address_cost, _) = self
            .client
            .graph_entry_put(address.clone(), payment_option.clone())
            .await?;
        let cost = payload_cost
            .checked_add(seal_cost)
            .unwrap_or(AttoTokens::zero())
            .checked_add(address_cost)
            .unwrap_or(AttoTokens::zero());
        Ok(Package {
            address,
            seal,
            payload,
            cost,
        })
    }
}

#[cfg(test)]

mod tests {
    use super::*;
    use crate::client::ConnectionType;
    use autonomi::{Bytes, GraphEntryAddress};

    #[tokio::test]
    #[ignore]
    async fn package_create() -> Result<(), PostemError> {
        let client = PostemClient::init(ConnectionType::Local).await?;
        let payment_option = client.get_payment_option("").await?;
        let public_key = SecretKey::random().public_key();
        let location = SecretKey::random();
        let payload = Bytes::from("Dear world");
        let package = client
            .package_create(
                &location,
                &public_key,
                payload.clone(),
                payment_option.clone(),
            )
            .await;
        assert!(package.is_ok());
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let package = client
            .package_create(
                &location,
                &public_key,
                payload.clone(),
                payment_option.clone(),
            )
            .await;
        assert_eq!(
            package,
            Err(PostemError::NameAlreadyExists(
                GraphEntryAddress::new(location.public_key()).to_hex(),
                None
            ))
        );
        Ok(())
    }
}
