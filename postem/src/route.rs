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

use autonomi::pointer::PointerTarget;
use autonomi::{GraphEntryAddress, PointerAddress, SecretKey};

use crate::addressee::PostemName;
use crate::{addressee::PostemBase, client::PostemClient, error::PostemError};

#[derive(Clone, Debug)]
pub struct Route {
    base: PostemBase,
    current_location: SecretKey,
}
impl Route {
    pub fn new(base: PostemBase, current_location: SecretKey) -> Route {
        Route {
            base,
            current_location,
        }
    }

    /// update current_location to the next location according to the base derivation index
    pub fn next(&mut self) {
        self.current_location = self
            .current_location
            .derive_child(&self.base.derivation_index().into_bytes());
    }

    pub fn base(&self) -> PostemBase {
        self.base.clone()
    }
}

impl PostemClient {
    pub async fn route_get(
        &self,
        name: PostemName,
        derive_from_base: bool,
    ) -> Result<Route, PostemError> {
        // let address = name.derive_key()?;
        // let graph_entry_address = GraphEntryAddress::new(address.public_key());
        // let base =
        //     PostemBase::from_graph_entry(self.client.graph_entry_get(&graph_entry_address).await?)?;
        let base = self.base_get(name.clone()).await?;
        let graph_entry_address = base.graph_entry_address();
        let public_key = base.public_key();
        let mut current_location = name.derive_key()?;
        if !derive_from_base {
            let last_received = self
                .client
                .pointer_get(&PointerAddress::new(public_key))
                .await?;
            // if not pointing at base goto last received and if that is valid use at start location
            let current_graph_entry = if last_received.target().xorname()
                != PointerTarget::GraphEntryAddress(graph_entry_address).xorname()
                && let PointerTarget::GraphEntryAddress(last_received_address) =
                    PointerTarget::GraphEntryAddress(graph_entry_address)
            {
                if let Ok(current_graph_entry) =
                    self.client.graph_entry_get(&last_received_address).await
                {
                    if let Ok(location) = SecretKey::from_bytes(current_graph_entry.content) {
                        current_location = location;
                    }
                }
                // what errors happen if you try to get a graph entry and something else is there or nothing is there?
                // GraphError:AlreadyExists for the former GraphEntry::Serialiation for the later
            };
        }

        Ok(Route::new(base.clone(), current_location))
    }

    pub async fn location_used(&self, route: Route) -> Result<bool, PostemError> {
        Ok(self
            .check_if_public_key_used(&route.current_location.public_key())
            .await?)
    }

    // pub async fn location_get_pacakge(&self, route: &Route) -> Result<Package, PostemError> {
    //     let
    // }

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

    use autonomi::client::key_derivation::DerivationIndex;
    use autonomi::graph::GraphError;
    use autonomi::{Bytes, GraphEntryAddress, Pointer};

    use super::*;
    use crate::client::ConnectionType;

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
        let route = Route::new(addressee.base(), addressee.address().derive_key()?);
        let next_location = client.location_get_available(route.clone()).await?;
        assert_eq!(
            next_location.to_hex(),
            addressee
                .address()
                .derive_key()?
                .derive_child(&index.into_bytes())
                .to_hex()
        );
        let (package, _) = client
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

    #[tokio::test]
    async fn get_route() {
        // Assumes run from freshly started local client
        let client = PostemClient::init(ConnectionType::Local).await.unwrap();
        let payment_option = client.get_payment_option("").await.unwrap();
        let addressee = client
            .addressee_create("another.test.address", payment_option.clone(), None)
            .await
            .unwrap();
        let route = client.route_get(addressee.address(), false).await.unwrap();
        assert_eq!(
            route.current_location.to_bytes(),
            addressee.address().derive_key().unwrap().to_bytes()
        );
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        // test currentl_location after adding a package
        // let next_location = client.location_get_available(route).await.unwrap();
        let (package, _) = client
            .package_post(addressee.address(), Bytes::from("Hello"), payment_option)
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let route = client.route_get(addressee.address(), false).await.unwrap();
        // need to make recieveing method that updates last_received before further tests
        // assert_eq!(route.current_location.to_bytes(), package.address().content);
        assert_eq!(
            route.current_location.to_bytes(),
            addressee.address().derive_key().unwrap().to_bytes()
        );
        // after adding a pointer (ie not valid pacakge)
        // after adding another package.
    }

    // #[tokio::test]
    // #[ignore]
    // async fn test_forked_base() {
    //     // only run after previous two test cause a fork
    //     let client = PostemClient::init(ConnectionType::Local).await.unwrap();
    //     let payment_option = client.get_payment_option("").await.unwrap();
    //     let name = PostemName::create("test.address").unwrap();
    //     let route = client.route_get(name, false).await.unwrap();
    //     eprintln!("{}", route.base.public_key().to_hex());
    //     let graph_result = client
    //         .client
    //         .graph_entry_get(&route.base.graph_entry_address())
    //         .await;
    //     eprintln!("{:?}", graph_result);
    // }
}
