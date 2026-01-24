/*
Copyright (C) 2025-2026 Postem

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
use autonomi::graph::GraphError;
use autonomi::{AttoTokens, Bytes, Chunk, ChunkAddress, GraphEntry, PublicKey, SecretKey, XorName};

use crate::addressee::PostemName;
use crate::client::PostemClient;
use crate::error::PostemError;

/// the number of tims the crate will retry posting a package where the non address components
/// have already been created, molstl likely because something else has been posted to the
/// next available location sinc it was fetched, after this it will fai with ChasingItsTail
/// which will contain the seal so that an application can attempt to handle this further
const NUMBER_OF_RETRIES_TO_POST_SEMI_CREATED_PACKAGE: u32 = 10;

#[derive(Debug, PartialEq)]
pub enum PackageState {
    Open,
    Sealed,
}

/// A package to be deilvered consiting of it address which will be derived from the addressee
/// the payload arbitary data in Bytes and the seal the encrypted hex of the datamap of the
/// payload
#[derive(Clone, Debug, PartialEq)]
pub struct Package {
    address: GraphEntry,
    seal: Chunk,            // Contains encrypted hex of data map of payload
    payload: Option<Bytes>, // a retrieved package that is yet to be open will be None
}
impl Package {
    pub fn address(&self) -> GraphEntry {
        self.address.clone()
    }

    pub fn public_key(&self) -> PublicKey {
        self.address.owner
    }

    pub fn status(&self) -> PackageState {
        if self.payload.is_some() {
            PackageState::Open
        } else {
            PackageState::Sealed
        }
    }

    pub fn seal(&self) -> Chunk {
        self.seal.clone()
    }

    pub fn payload(&self) -> Option<Bytes> {
        self.payload.clone()
    }

    pub fn set_payload(&mut self, payload: Bytes) {
        self.payload = Some(payload);
    }

    /// clones the pacakge and adds a payload
    /// for use when opening packages which won't initally have a payload
    pub fn clone_with_new_payload(&self, payload: Bytes) -> Package {
        let mut package = self.clone();
        package.set_payload(payload);
        package
    }
}
impl PostemClient {
    pub async fn package_create(
        &self,
        location: &SecretKey,
        public_key: &PublicKey,
        payload: &Bytes,
        payment_option: &PaymentOption,
        seal: Option<Chunk>, // so if function previously create seal but couldn't do the graph it can try again
    ) -> Result<(Package, AttoTokens), PostemError> {
        let (seal, addr, payload_cost, seal_cost) = if seal.is_none() {
            if payload.len() < 3 {
                return Err(PostemError::PayloadTooSmall);
            }
            let (payload_cost, data_map) = self
                .client
                .data_put(payload.clone(), payment_option.clone())
                .await?;
            let seal = Chunk::new(
                // Bytes::from(
                // public_key.encrypt(data_map.to_hex()).to_bytes(),
                Bytes::from(public_key.encrypt(data_map.0.value).to_bytes()),
            );
            let (seal_cost, addr) = self.client.chunk_put(&seal, payment_option.clone()).await?;
            (seal, addr, payload_cost, seal_cost)
        } else {
            (
                seal.clone().unwrap(),
                *seal.unwrap().address(),
                AttoTokens::zero(),
                AttoTokens::zero(),
            )
        };
        let address = GraphEntry::new(
            &location,
            vec![],
            location.to_bytes(), // location needs to be here so next one can be derived if jupmed to
            vec![(public_key.clone(), addr.xorname().0)],
        );
        let (address_cost, _) = match self
            .client
            .graph_entry_put(address.clone(), payment_option.clone())
            .await
        {
            Ok(ok) => ok,
            Err(GraphError::AlreadyExists(_)) => return Err(PostemError::ChasingItsTail(seal)),
            Err(e) => Err(e)?,
        };
        let cost = payload_cost
            .checked_add(seal_cost)
            .unwrap_or(AttoTokens::zero())
            .checked_add(address_cost)
            .unwrap_or(AttoTokens::zero());
        Ok((
            Package {
                address,
                seal,
                payload: Some(payload.clone()),
            },
            cost,
        ))
    }

    pub async fn package_cost(
        &self,
        payload: Bytes,
        // location_pk: &PublicKey,
        number_of_recipients: usize,
    ) -> Result<AttoTokens, PostemError> {
        if payload.len() < 3 {
            return Err(PostemError::PayloadTooSmall);
        }
        let cost = self.client.data_cost(payload).await?;
        let mut posting_cost = AttoTokens::zero();
        if number_of_recipients > 0 {
            // hmm this will probably return 0 is someone actually stores a chunk full of zeros
            posting_cost = posting_cost
                .checked_add(
                    self.client
                        .chunk_cost(
                            Chunk::new(Bytes::copy_from_slice(&[0u8; Chunk::MAX_RAW_SIZE]))
                                .address(),
                        )
                        .await?,
                )
                .unwrap_or(AttoTokens::zero());
            posting_cost = posting_cost
                .checked_add(
                    self.client
                        .graph_entry_cost(&SecretKey::random().public_key())
                        .await?,
                )
                .unwrap_or(AttoTokens::zero());
            // multiply posting cost by number of recipients
            for _ in 1..number_of_recipients {
                posting_cost = posting_cost
                    .checked_add(posting_cost)
                    .unwrap_or(AttoTokens::zero());
            }
        };
        Ok(cost.checked_add(posting_cost).unwrap_or(AttoTokens::zero()))
    }

    pub async fn package_post(
        &self,
        postem_name: PostemName,
        content: Bytes,
        payment_option: PaymentOption,
    ) -> Result<(Package, AttoTokens), PostemError> {
        let mut route = self.route_get(postem_name, false).await?;
        let base = route.base();
        let mut seal = None;
        for i in 0..NUMBER_OF_RETRIES_TO_POST_SEMI_CREATED_PACKAGE {
            let location = self.location_get_available(&mut route).await?;
            // if location is used (as it may have been since the route was got) then it needs to find
            // next availabel location and try again...probably several times
            // if it fails it should pass back the seal so application can choose to reuse it in it's
            // handeling of the addresse being to busy to manager to post something
            match self
                .package_create(
                    &location,
                    &base.public_key(),
                    &content,
                    &payment_option,
                    seal,
                )
                .await
            {
                Ok(ok) => return Ok(ok),
                Err(PostemError::ChasingItsTail(returned_seal)) => seal = Some(returned_seal),
                Err(e) => return Err(e),
            }
        }
        Err(PostemError::ChasingItsTail(
            seal.expect("Seal should already exist by this point"),
        ))
    }

    /// Uses the address graph entry to retrieve the encryped data map and return a sealed package
    /// plus the public key which will only be valid if it matches the bases public key
    pub async fn package_get(
        &self,
        address: GraphEntry,
    ) -> Result<(Package, PublicKey), PostemError> {
        if let Some((public_key, seal_xor)) = address.descendants.first() {
            let seal = self
                .client
                .chunk_get(&ChunkAddress::new(XorName(*seal_xor)))
                .await?;
            return Ok((
                Package {
                    address: address.clone(),
                    seal,
                    payload: None,
                },
                *public_key,
            ));
        }
        Err(PostemError::MissingSeal)
    }
}

#[cfg(test)]

mod tests {
    use super::*;
    use crate::addressee::PostemName;
    use crate::client::ConnectionType;
    use autonomi::Bytes;
    use autonomi::self_encryption::MAX_CHUNK_SIZE;

    #[tokio::test]
    async fn package_create() -> Result<(), PostemError> {
        let client = PostemClient::init(ConnectionType::Local).await?;
        let payment_option = client.get_payment_option("")?;
        let public_key = SecretKey::random().public_key();
        let location = SecretKey::random();
        let payload = Bytes::from("Dear world");
        let package = client
            .package_create(&location, &public_key, &payload, &payment_option, None)
            .await;
        assert!(package.is_ok());
        let seal = package.unwrap().0.seal();
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let package = client
            .package_create(&location, &public_key, &payload, &payment_option, None)
            .await;
        assert!(package.is_err());
        let package = client
            .package_create(
                &location,
                &public_key,
                &payload,
                &payment_option,
                Some(seal.clone()),
            )
            .await;
        assert_eq!(package, Err(PostemError::ChasingItsTail(seal)));
        Ok(())
    }

    #[tokio::test]
    async fn package_post_and_get() {
        let client = PostemClient::init(ConnectionType::Local).await.unwrap();
        let payment_option = client.get_payment_option("").unwrap();
        let name = SecretKey::random().to_hex();
        let postem_name = PostemName::create(&name).unwrap();
        client
            .addressee_create(&name, payment_option.clone(), None)
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let message = Bytes::from("Hello");
        let (package, _attos) = client
            .package_post(postem_name, message, payment_option)
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let got_package = client.package_get(package.clone().address).await.unwrap();
        assert_eq!(got_package.0.seal(), package.seal());
    }

    #[tokio::test]
    async fn package_cost() {
        let client = PostemClient::init(ConnectionType::Local).await.unwrap();
        let payload = Bytes::from("How much do I cost?");
        // let location_pk = SecretKey::random().public_key();
        let just_payload_cost = client.package_cost(payload.clone(), 0).await.unwrap();
        let cost_for_one = client.package_cost(payload.clone(), 1).await.unwrap();
        let cost_for_two = client.package_cost(payload, 2).await.unwrap();
        eprintln!(
            "Just payload: {}\nWith one recipient: {}\nWith two recipients: {}",
            just_payload_cost, cost_for_one, cost_for_two
        );
        let postage_cost = cost_for_one.checked_sub(just_payload_cost).unwrap();
        let calcualted_cost_for_two = cost_for_one.checked_add(postage_cost).unwrap();
        assert_eq!(cost_for_two, calcualted_cost_for_two);
    }
}
