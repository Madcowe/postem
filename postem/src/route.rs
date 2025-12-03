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

use autonomi::graph::GraphError;
use autonomi::{GraphEntry, GraphEntryAddress, SecretKey};

use crate::addressee::PostemName;
use crate::package::Package;
use crate::{addressee::PostemBase, client::PostemClient, error::PostemError};

#[derive(Clone, Debug)]
pub struct Route {
    base: PostemBase,
    last_received: SecretKey,
    current_location: SecretKey,
}
impl Route {
    pub fn new(base: PostemBase, last_received: SecretKey, current_location: SecretKey) -> Route {
        Route {
            base,
            last_received,
            current_location,
        }
    }

    /// update current_location to the next location according to the base derivation index
    pub fn next(&mut self) {
        self.current_location = self
            .current_location
            .derive_child(&self.base.derivation_index().into_bytes());
        // .derive_child(&self.base.derivation_index());
    }

    pub fn base(&self) -> PostemBase {
        self.base.clone()
    }

    pub fn current_location(&self) -> SecretKey {
        self.current_location.clone()
    }
}

impl PostemClient {
    pub async fn route_get(
        &self,
        name: PostemName,
        derive_from_base: bool,
    ) -> Result<Route, PostemError> {
        let base = self.base_get(name.clone()).await?;
        let graph_entry_address = base.graph_entry_address();
        // let public_key = base.public_key();
        let mut current_location = name.derive_key()?;
        if !derive_from_base {
            let last_received_address =
                GraphEntryAddress::new(self.base_get_last_received_public_key(&base).await?);
            if last_received_address != graph_entry_address {
                if let Ok(current_graph_entry) =
                    self.client.graph_entry_get(&last_received_address).await
                {
                    if let Ok(location) = SecretKey::from_bytes(current_graph_entry.content) {
                        current_location = location;
                    }
                }
                // what errors happen if you try to get a graph entry and something else is there or nothing is there?
                // GraphError::Serialization for the former and GraphError:GetError for the later
                // Trying to put to a location with something already there will give GraphError:AlreadyExists, even if not graph
            };
        }

        Ok(Route::new(
            base.clone(),
            current_location.clone(),
            current_location,
        ))
    }

    pub async fn location_used(&self, route: &Route) -> Result<bool, PostemError> {
        Ok(self
            .check_if_public_key_used(&route.current_location.public_key())
            .await?)
    }

    /// If graph has forks returns all forks as they if valid can all be considered as packages
    /// hence wraps succesful return in vector
    pub async fn location_get_package_addresses(
        &self,
        route: &Route,
    ) -> Result<Vec<GraphEntry>, PostemError> {
        match self
            .client
            .graph_entry_get(&GraphEntryAddress::new(route.current_location.public_key()))
            .await
        {
            Ok(graph_entry) => Ok(vec![graph_entry]),
            Err(GraphError::Fork(forks)) => Ok(forks),
            // Return empty vector if not graph entry...so should skip if another autonomi type
            Err(GraphError::Serialization(_)) => Ok(Vec::new()),
            // Should we deal with GraphError:GetError which would indicate it not existing
            // in theory this shouldn't happen if the calling function are correct so probably
            // just want error returned if this did happen
            Err(e) => return Err(e.into()),
        }
    }

    // potentially you might want to return enum an that could be a package or not valid along with
    // address so an application could know about the invalid locations
    pub async fn location_get_packages(
        &self,
        route: &Route,
    ) -> Result<Vec<(Package, SecretKey)>, PostemError> {
        let addresses = self.location_get_package_addresses(route).await?;
        let mut packages = Vec::new();
        for address in addresses {
            if let Ok((package, public_key)) = self.package_get(address).await {
                // Check if package is valid as will have public_key of addressee
                if route.base.public_key() == public_key {
                    packages.push((package, route.current_location.clone()));
                }
            }
        }
        Ok(packages)
    }

    /// Returns all packages along the route, whether this is everythig or just since last received
    /// depends on the value of derive_from_base when route_get was called
    /// note the route's current location is modifed as the route is traversed
    /// you probably don;t want to call this on a route that has already called another function
    /// that also does this (eg location_get_available) as you will miss most pacakges
    pub async fn route_get_packages(
        &self,
        route: &mut Route,
    ) -> Result<Vec<(Package, SecretKey)>, PostemError> {
        let mut packages = Vec::new();
        // skip if at last received as don't need to return pacakge if already receieved
        // or if derived from base will just be base address
        if route.current_location.to_hex() == route.last_received.to_hex() {
            route.next();
        }
        while self.location_used(&route).await? {
            packages.append(&mut self.location_get_packages(&route).await?);
            route.next();
        }
        Ok(packages)
    }

    /// Returns the next location on the route that has not been used, so a package may be posted
    /// note the route current location is modified as the route is traversed
    pub async fn location_get_available(
        &self,
        route: &mut Route,
    ) -> Result<SecretKey, PostemError> {
        while self.location_used(&route).await? {
            route.next();
        }
        Ok(route.current_location.clone())
    }
}

#[cfg(test)]
mod tests {

    use autonomi::client::key_derivation::DerivationIndex;
    use autonomi::{Bytes, GraphEntryAddress, Pointer};

    use super::*;
    use crate::client::ConnectionType;

    #[tokio::test]
    // Assumes run from freshly started local client
    async fn location_get_available() -> Result<(), PostemError> {
        let client = PostemClient::init(ConnectionType::Local).await?;
        let payment_option = client.get_payment_option("")?;
        let (addressee, _) = client
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
        let start_location = addressee.address().derive_key().unwrap();
        let mut route = Route::new(addressee.base(), start_location.clone(), start_location);
        let next_location = client.location_get_available(&mut route).await?;
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
                &Bytes::from("Hello world!"),
                &payment_option,
                None,
            )
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let next_location = client.location_get_available(&mut route).await.unwrap();
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
        let next_location = client.location_get_available(&mut route).await.unwrap();
        assert_eq!(next_location.to_hex(), location_after_next.to_hex());
        Ok(())
    }

    #[tokio::test]
    async fn route_get() {
        // Assumes run from freshly started local client
        let client = PostemClient::init(ConnectionType::Local).await.unwrap();
        let payment_option = client.get_payment_option("").unwrap();
        let (addressee, _) = client
            .addressee_create("another.test.address", payment_option.clone(), None)
            .await
            .unwrap();
        let route = client.route_get(addressee.address(), false).await.unwrap();
        assert_eq!(
            route.current_location.to_hex(),
            addressee.address().derive_key().unwrap().to_hex()
        );
    }

    #[tokio::test]
    async fn route_get_packages() {
        let client = PostemClient::init(ConnectionType::Local).await.unwrap();
        let payment_option = client.get_payment_option("").unwrap();
        let name = SecretKey::random().to_hex();
        let (addressee, _) = client
            .addressee_create(&name, payment_option.clone(), None)
            .await
            .unwrap();
        // let addressee = client
        //     .addressee_create("yet.another.test.address", payment_option.clone(), None)
        //     .await
        //     .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let mut route = client.route_get(addressee.address(), true).await.unwrap();
        let packages = client.route_get_packages(&mut route).await.unwrap();
        assert!(packages.is_empty());
        let message = Bytes::from("Hello");
        let (_package, _attos) = client
            .package_post(addressee.address(), message.clone(), payment_option.clone())
            .await
            .unwrap();
        let (_package, _attos) = client
            .package_post(addressee.address(), message.clone(), payment_option.clone())
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let mut route = client.route_get(addressee.address(), true).await.unwrap();
        let packages = client.route_get_packages(&mut route).await.unwrap();
        assert_eq!(packages.len(), 2);
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
