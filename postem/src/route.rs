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

#[derive(Clone)]
pub struct Route {
    base: PostemBase,
    current_location: SecretKey,
}
impl Route {
    pub fn new(&self, base: PostemBase, current_location: SecretKey) -> Route {
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
}

impl PostemClient {
    pub async fn location_used(&self, route: Route) -> Result<bool, PostemError> {
        Ok(self
            .check_if_public_key_used(&route.current_location.public_key())
            .await?)
    }

    /// Returns the next location on the route that has not been used, so a package may be posted
    pub async fn location_get_available(&self, mut route: Route) -> Result<SecretKey, PostemError> {
        while !self.location_used(route.clone()).await? {
            route.next();
        }
        Ok(route.current_location)
    }
}
