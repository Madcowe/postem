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
use autonomi::client::payment::PaymentOption;
use autonomi::pointer::PointerTarget;
use autonomi::{
    AttoTokens, Bytes, GraphEntry, GraphEntryAddress, PointerAddress, PublicKey, SecretKey,
};
use blsttc::rand;

use crate::{client::PostemClient, error::PostemError};

/// Hex key of base (not) secret key of postem derived names
pub const POSTEM_DERIVED_KEY_BASE: &str =
    "00000000000000000000000000000000000000000000000000000003C2BABF81";

/// Characters that are not allowed in an addressee name
/// Currenty the most common deliminators so as too be able to differntiate mutiple addressees
const POSTEM_NAME_INVALID_CHARS: [char; 2] = [',', ';'];

/// The unique name of an addressee, full stops (.) deliminate domains.
#[derive(Clone, Debug, PartialEq)]
pub struct PostemName(Vec<String>);
impl PostemName {
    pub fn create(name: &str) -> Result<PostemName, PostemError> {
        for char in POSTEM_NAME_INVALID_CHARS {
            if name.contains(char) {
                Err(PostemError::NameContainsInvalidCharatcers(char))?
            }
        }
        if name.replace(".", "").len() == 0 {
            Err(PostemError::BlankName)?
        }
        let doms: Vec<_> = name
            .trim()
            .split(".")
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        Ok(PostemName(doms))
    }

    pub fn derive_key(&self) -> Result<SecretKey, PostemError> {
        let mut key = SecretKey::from_hex(&POSTEM_DERIVED_KEY_BASE)?;
        for domain in self.0.clone() {
            key = key.derive_child(&Bytes::copy_from_slice(domain.as_bytes()));
        }
        Ok(key)
    }
}

impl PostemClient {
    pub async fn check_if_public_key_used(
        &self,
        public_key: &PublicKey,
    ) -> Result<bool, PostemError> {
        Ok(self
            .client
            .graph_entry_check_existence(&GraphEntryAddress::new(public_key.clone()))
            .await?)
    }
}

/// The base of a postem address from which the location of all received pacakges can be derived
/// the first entry in parents is the (address/public key) of a pointer to the last received package
/// the key pair used to create said pointer is also use for encrypton, the content is the
/// derivation index that will be used to generate all subseqent package locations.
#[derive(Clone, Debug, PartialEq)]
pub struct PostemBase(GraphEntry);
impl PostemBase {
    pub fn from_graph_entry(graph_entry: GraphEntry) -> Result<PostemBase, PostemError> {
        match graph_entry.parents.first() {
            None => return Err(PostemError::NotValidPostemBase()),
            Some(_) => Ok(PostemBase(graph_entry)),
        }
    }

    pub fn derivation_index(&self) -> DerivationIndex {
        DerivationIndex::from_bytes(self.0.content)
    }

    /// This is both the key used for encrypton and to create the address of last received pointer
    pub fn public_key(&self) -> PublicKey {
        self.0
            .parents
            .first()
            .expect("Postem base should always have public key in parents first entry")
            .clone()
    }

    pub fn graph_entry(&self) -> GraphEntry {
        self.0.clone()
    }
}

impl PostemClient {
    pub async fn base_create(
        &self,
        address: SecretKey,
        last_received_pk: &PublicKey,
        payment_option: PaymentOption,
    ) -> Result<PostemBase, PostemError> {
        let index = DerivationIndex::random(&mut rand::thread_rng());
        let base = GraphEntry::new(
            &address,
            vec![last_received_pk.clone()],
            index.into_bytes(),
            vec![],
        );
        self.client
            .graph_entry_put(base.clone(), payment_option)
            .await?;
        Ok(PostemBase(base))
    }

    pub async fn base_get(&self, name: PostemName) -> Result<PostemBase, PostemError> {
        PostemBase::from_graph_entry(
            self.client
                .graph_entry_get(&GraphEntryAddress::new(name.derive_key()?.public_key()))
                .await?,
        )
    }
}

/// The addressee which can receive packages. Consisting of the address where the base is located,
/// the base from which the location of pacakges sent to it can be derived, and last received a
/// pointer to the latest package received. Received in this context means the owner of the
/// address has followed the chain of location from the base up to this point...there may well be
/// pacakges that have been sent since they last checked beyond this location and the package
/// at the location may not be valid.
#[derive(Debug, PartialEq)]
pub struct Addressee {
    address: PostemName,
    secret_key: SecretKey,
    base: PostemBase,
    last_received: PointerAddress,
}
impl PostemClient {
    pub async fn addressee_create(
        &self,
        name: &str,
        payment_option: PaymentOption,
        last_received_key: Option<SecretKey>,
    ) -> Result<Addressee, PostemError> {
        let address = PostemName::create(&name)?;
        let base_sk = address.derive_key()?;
        let base_pk = base_sk.public_key();
        let target = PointerTarget::GraphEntryAddress(GraphEntryAddress::new(base_pk));
        if self.check_if_public_key_used(&base_pk).await? {
            return Err(PostemError::NameAlreadyExists(name.to_string(), None));
        }
        let (secret_key, pointer_address) = match last_received_key {
            Some(secret_key) => {
                self.client.pointer_update(&secret_key, target).await?;
                (
                    secret_key.clone(),
                    PointerAddress::new(secret_key.public_key()),
                )
            }
            None => {
                let secret_key = SecretKey::random();
                let (_, pointer_address) = self
                    .client
                    .pointer_create(&secret_key, target, payment_option.clone())
                    .await?;
                (secret_key, pointer_address)
            }
        };
        let base = match self
            .base_create(base_sk, &secret_key.public_key(), payment_option.clone())
            .await
        {
            Ok(base) => base,
            // Returns pointer owner so pointer could be reused with a different name if name taken
            Err(PostemError::NameAlreadyExists(..)) => Err(PostemError::NameAlreadyExists(
                name.to_string(),
                Some(secret_key.clone()),
            ))?,
            Err(e) => Err(e)?,
        };
        Ok(Addressee {
            address,
            secret_key,
            base,
            last_received: pointer_address,
        })
    }

    pub async fn addressee_cost(&self, name: &str) -> Result<AttoTokens, PostemError> {
        let address = PostemName::create(&name)?;
        let graph_key = address.derive_key()?.public_key();
        let graph_entry_cost = self.client.graph_entry_cost(&graph_key);
        let pointer_key = SecretKey::random().public_key();
        let pointer_cost = self.client.pointer_cost(&pointer_key);
        let cost = graph_entry_cost
            .await?
            .checked_add(pointer_cost.await?)
            .unwrap_or(AttoTokens::zero());
        Ok(cost)
    }
}

impl Addressee {
    pub fn base(&self) -> PostemBase {
        self.base.clone()
    }

    pub fn last_received(&self) -> PointerAddress {
        self.last_received
    }

    pub fn secret_key(&self) -> SecretKey {
        self.secret_key.clone()
    }

    pub fn address(&self) -> PostemName {
        self.address.clone()
    }
}

#[cfg(test)]
mod tests {

    use autonomi::Client;

    use super::*;
    use crate::client::ConnectionType;

    #[test]
    fn postem_name() {
        let result = PostemName::create("");
        assert_eq!(result, Err(PostemError::BlankName));
        let result = PostemName::create(".");
        assert_eq!(result, Err(PostemError::BlankName));
        let result = PostemName::create("...");
        assert_eq!(result, Err(PostemError::BlankName));
        let result = PostemName::create(";");
        assert_eq!(result, Err(PostemError::NameContainsInvalidCharatcers(';')));
        let result = PostemName::create("a;a");
        assert_eq!(result, Err(PostemError::NameContainsInvalidCharatcers(';')));
        let result = PostemName::create("a.;.a");
        assert_eq!(result, Err(PostemError::NameContainsInvalidCharatcers(';')));
        let result = PostemName::create("dom.dom.dom.d,m.dom");
        assert_eq!(result, Err(PostemError::NameContainsInvalidCharatcers(',')));
        let result = PostemName::create("dom");
        assert_eq!(result, Ok(PostemName(vec!["dom".to_string()])));
        let result = PostemName::create("dom.dom");
        assert_eq!(
            result,
            Ok(PostemName(vec!["dom".to_string(), "dom".to_string()]))
        );
        let result = PostemName::create("dom..dom");
        assert_eq!(
            result,
            Ok(PostemName(vec!["dom".to_string(), "dom".to_string()]))
        );
        let result = PostemName::create(" dom..dom   ");
        assert_eq!(
            result,
            Ok(PostemName(vec!["dom".to_string(), "dom".to_string()]))
        );
    }

    #[tokio::test]
    #[ignore]
    // Assumes run from freshly started local client
    async fn check_if_public_key_used() -> Result<(), PostemError> {
        let client = PostemClient::init(ConnectionType::Local).await?;
        let name = PostemName::create("dom.dom")?;
        let secret_key = name.derive_key()?;
        let public_key = secret_key.public_key();
        let bool = client.check_if_public_key_used(&public_key).await?;
        // This will fail if local clinet data has not been reset
        assert_eq!(bool, false);
        let target = PointerTarget::PointerAddress(PointerAddress::new(public_key));
        let payment_option = client.get_payment_option("").await?;
        client
            .client
            .pointer_create(&secret_key, target, payment_option)
            .await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let bool = client.check_if_public_key_used(&public_key).await?;
        assert_eq!(bool, true);
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    // Assumes run from freshly started local client
    async fn create_addressee() -> Result<(), PostemError> {
        let client = PostemClient::init(ConnectionType::Local).await?;
        let payment_option = client.get_payment_option("").await?;
        let name = "my.address";
        let estimate = client.addressee_cost(&name).await?;
        eprintln!("Estimate: {:?}", estimate);
        let addressee = client
            .addressee_create(name, payment_option.clone(), None)
            .await?;
        assert_eq!(addressee.address, PostemName::create(&name)?);
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let addressee_result = client
            .addressee_create(name, payment_option.clone(), None)
            .await;
        assert_eq!(
            addressee_result,
            Err(PostemError::NameAlreadyExists(name.to_string(), None))
        );
        // test with existing last_recieved
        let target = PointerTarget::GraphEntryAddress(addressee.base.0.address());
        let secret_key = SecretKey::random();
        let (_, _) = client
            .client
            .pointer_create(&secret_key, target, payment_option.clone())
            .await?;
        let address = PostemName::create("my.address")?;
        let base_sk = address.derive_key()?;
        let base = client
            .base_create(base_sk, &secret_key.public_key(), payment_option.clone())
            .await;
        assert_eq!(
            base,
            Err(PostemError::NameAlreadyExists(
                addressee.base.0.address().to_hex(),
                None
            ))
        );
        Ok(())
    }

    // it seems the particular address can have signifcant varation in quote
    // weirdly how.much.do.I.cost always returned the same when I ran this but my.address didn't???
    // #[tokio::test]
    // async fn costs() {
    //     // let client = PostemClient::init(ConnectionType::Local).await.unwrap();
    //     let client = PostemClient::init(ConnectionType::Antnet).await.unwrap();
    //     let name = "how.much.do.I.cost";
    //     // let name = "my.address";
    //     let name_key = PostemName::create(name).unwrap().derive_key().unwrap();
    //     let random_key = SecretKey::random();
    //     let mut costs = vec![];
    //     costs.push(
    //         client
    //             .client
    //             .graph_entry_cost(&name_key.public_key())
    //             .await
    //             .unwrap(),
    //     );
    //     costs.push(
    //         client
    //             .client
    //             .graph_entry_cost(&random_key.public_key())
    //             .await
    //             .unwrap(),
    //     );
    //     tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    //     costs.push(
    //         client
    //             .client
    //             .graph_entry_cost(&name_key.public_key())
    //             .await
    //             .unwrap(),
    //     );
    //     costs.push(
    //         client
    //             .client
    //             .graph_entry_cost(&random_key.public_key())
    //             .await
    //             .unwrap(),
    //     );
    //     eprintln!("{:?}", costs);
    // }
}
