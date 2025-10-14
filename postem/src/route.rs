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

use autonomi::client::key_derivation::DerivationIndex;
use autonomi::{GraphEntryAddress, SecretKey};

use crate::{addressee::PostemBase, client::PostemClient, error::PostemError, package::Package};

#[derive(Clone, Debug)]
pub struct Route {
    base: PostemBase,
    current_location: SecretKey,
    index: DerivationIndex,
}
impl Route {
    pub fn new(base: PostemBase, current_location: SecretKey, index: DerivationIndex) -> Route {
        Route {
            base,
            current_location,
            index,
        }
    }

    /// update current_location to the next location according to the base derivation index
    pub fn next(&mut self) {
        self.current_location = self
            .current_location
            .derive_child(&self.base.derivation_index().into_bytes());
        eprintln!("{:?}", self.current_location.to_hex());
    }
}

impl PostemClient {
    pub async fn location_used(&self, route: Route) -> Result<bool, PostemError> {
        Ok(self
            .check_if_public_key_used(&route.current_location.public_key())
            .await?)
    }

    /// Returns the next location on the route that has not been used, so a package may be posted
    pub async fn location_get_available(&self, mut route: Route) -> Result<SecretKey, PostemError> {
        while self.location_used(route.clone()).await? {
            route.next();
        }
        Ok(route.current_location)
    }
}

#[cfg(test)]
mod tests {

    use autonomi::{Bytes, GraphEntryAddress, Pointer};

    use super::*;
    use crate::{addressee, client::ConnectionType};

    #[tokio::test]
    #[ignore]
    // Assumes run from freshly started local client
    async fn location_get_available() -> Result<(), PostemError> {
        let client = PostemClient::init(ConnectionType::Local).await?;
        let payment_option = client.get_payment_option("").await?;
        let addressee = client
            .addressee_create("test.address", payment_option.clone(), None)
            .await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let graph_entry = client
            .client
            .graph_entry_get(&GraphEntryAddress::new(
                addressee.address().derive_key()?.public_key(),
            ))
            .await?;
        let index = DerivationIndex::from_bytes(graph_entry.content);
        let route = Route::new(addressee.base(), addressee.address().derive_key()?, index);
        let next_location = client.location_get_available(route.clone()).await?;
        assert_eq!(
            next_location.to_hex(),
            addressee
                .address()
                .derive_key()?
                .derive_child(&index.into_bytes())
                .to_hex()
        );
        let package = client
            .package_create(
                &next_location,
                graph_entry
                    .parents
                    .first()
                    .expect("Base should always have a first item in parents"),
                Bytes::from("Hello world!"),
                payment_option.clone(),
            )
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let next_location = client.location_get_available(route.clone()).await.unwrap();
        assert_eq!(
            next_location.to_hex(),
            SecretKey::from_bytes(package.address().content)
                .unwrap()
                .derive_child(&index.into_bytes())
                .to_hex()
        );
        // Test if non package put at address
        let location_after_next = next_location.derive_child(&index.into_bytes());
        let pointer = Pointer::new(
            &next_location,
            0,
            autonomi::pointer::PointerTarget::GraphEntryAddress(graph_entry.address()),
        );
        client
            .client
            .pointer_put(pointer, payment_option.clone())
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let next_location = client.location_get_available(route.clone()).await.unwrap();
        assert_eq!(next_location.to_hex(), location_after_next.to_hex());
        Ok(())
    }
}
