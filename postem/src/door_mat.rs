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
use crate::{Addressee, Package, PostemClient, PostemError, addressee::PostemName};
use autonomi::SecretKey;

/// A collection of all valid open pacakges for an addressee
pub struct Doormat {
    addressee: Addressee,
    items: Vec<Package>,
}
impl Doormat {
    /// Recreate doormat from localy saved resources rather than download in entirety
    pub fn recreate(addressee: Addressee, items: Vec<Package>) -> Doormat {
        Doormat { addressee, items }
    }

    pub fn addressee(&self) -> Addressee {
        self.addressee.clone()
    }

    pub fn items(&self) -> Vec<Package> {
        self.items.clone()
    }
}

impl PostemClient {
    pub async fn doormat_init(
        &mut self,
        secret_key: SecretKey,
        name: &str,
    ) -> Result<Doormat, PostemError> {
        let name = PostemName::create(name)?;
        let mut addressee = self.addressee_get(secret_key, name).await?;
        // let mut route = self.route_get(name, true).await?;
        let packages = self.addressee_get_packages(&mut addressee, true).await?;
        let items = self
            .addressee_inspect_packages(&mut addressee, &packages)
            .await?;
        Ok(Doormat { addressee, items })
    }

    /// check if any new pacakges and downloads returns true if this is the case
    pub async fn doormat_update(&mut self, doormat: &mut Doormat) -> Result<bool, PostemError> {
        let mut new_items = false;
        let mut new_packages = self
            .addressee_get_packages(&mut doormat.addressee(), false)
            .await?;
        if !new_packages.is_empty() {
            doormat.items.append(&mut new_packages);
            new_items = true;
        }
        Ok(new_items)
    }
}
