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

use autonomi::client::data_types::chunk::DataMapChunk;
use autonomi::client::key_derivation::DerivationIndex;
use autonomi::client::payment::PaymentOption;
use autonomi::graph::GraphError;
use autonomi::pointer::PointerTarget;
use autonomi::{
    AttoTokens, Bytes, Chunk, GraphEntry, GraphEntryAddress, PointerAddress, PublicKey, SecretKey,
};
use blsttc::Ciphertext;
use blsttc::rand;

use crate::{Package, client::PostemClient, error::PostemError};

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

    pub fn graph_entry_address(&self) -> GraphEntryAddress {
        self.0.address()
    }

    /// If there is a fork on the graph entry of a base this function arbitarily but
    /// determalistically picks ones so there is always only one valid addressee of a postem
    /// address...as I don't know if the order of forks in the vector would always be the same
    /// if a user is unfortunate enough for this to have happened they may end up paying for
    /// a non functinal address if this resolves to a differnt graph then the one they created
    pub fn resolve_fork(forks: Vec<GraphEntry>) -> GraphEntry {
        let mut public_key_bytes = [0u8; 48];
        let mut index_to_use = 0;
        for (index, fork) in forks.iter().enumerate() {
            if let Some(public_key) = fork.parents.first() {
                // find the highest value of first entry of parents
                if public_key.to_bytes() > public_key_bytes {
                    public_key_bytes = public_key.to_bytes();
                    index_to_use = index;
                }
            }
        }
        forks
            .get(index_to_use)
            .expect("Fork error should always return a non empty vector")
            .clone()
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
            match self
                .client
                .graph_entry_get(&GraphEntryAddress::new(name.derive_key()?.public_key()))
                .await
            {
                Ok(graph_entry) => graph_entry,
                Err(GraphError::Fork(forks)) => PostemBase::resolve_fork(forks),
                Err(e) => return Err(e.into()),
            },
        )
    }

    pub async fn base_get_from_public_key(
        &self,
        public_key: PublicKey,
    ) -> Result<PostemBase, PostemError> {
        PostemBase::from_graph_entry(
            match self
                .client
                .graph_entry_get(&GraphEntryAddress::new(public_key))
                .await
            {
                Ok(graph_entry) => graph_entry,
                Err(GraphError::Fork(forks)) => PostemBase::resolve_fork(forks),
                Err(e) => return Err(e.into()),
            },
        )
    }
}

/// The addressee which can receive packages. Consisting of the address where the base is located,
/// the base from which the location of pacakges sent to it can be derived, and last received a
/// pointer to the latest package received. Received in this context means the owner of the
/// address has followed the chain of location from the base up to this point...there may well be
/// pacakges that have been sent since they last checked beyond this location and the package
/// at the location may not be valid.
#[derive(Clone, Debug, PartialEq)]
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

    pub async fn addressee_get(
        &self,
        secret_key: SecretKey,
        address: PostemName,
    ) -> Result<Addressee, PostemError> {
        let base = self.base_get(address.clone()).await?;
        let last_received = match base.0.parents.first() {
            Some(public_key) => PointerAddress::new(*public_key),
            None => return Err(PostemError::NotValidPostemBase()),
        };
        Ok(Addressee {
            address,
            secret_key,
            base,
            last_received,
        })
    }

    pub async fn addressee_set_last_received(
        &mut self,
        addressee: &mut Addressee,
        location_graph_address: GraphEntryAddress,
    ) -> Result<(), PostemError> {
        let target = PointerTarget::GraphEntryAddress(location_graph_address);
        self.client
            .pointer_update(&addressee.secret_key, target)
            .await?;
        Ok(())
    }

    /// Get packages and sets last recieved
    pub async fn addressee_inspect_packages(
        &mut self,
        addressee: &mut Addressee,
        packages: &Vec<Package>,
    ) -> Result<Vec<Package>, PostemError> {
        let open_pacakges = self.packages_open(packages, addressee.secret_key()).await?;
        if let Some(package) = open_pacakges.last() {
            self.addressee_set_last_received(
                addressee,
                GraphEntryAddress::new(package.address().owner),
            )
            .await?;
        }
        Ok(open_pacakges)
    }

    pub async fn addressee_get_packages(
        &mut self,
        addressee: &mut Addressee,
        derive_from_base: bool,
    ) -> Result<Vec<Package>, PostemError> {
        let mut route = self
            .route_get(addressee.address(), derive_from_base)
            .await?;
        eprintln!(
            "sk: {:?}\npk: {:?}",
            route.current_location().to_hex(),
            route.current_location().public_key().to_hex()
        );
        let packages = self.route_get_packages(route.clone()).await?;
        eprintln!(
            "pk of location pacakge should be stored at {}",
            self.location_get_available(route)
                .await
                .unwrap()
                .public_key()
                .to_hex()
        );
        eprintln!("Packages received: {}", packages.len());
        if let Some((_, last_location)) = packages.last() {
            let target = PointerTarget::GraphEntryAddress(GraphEntryAddress::new(
                last_location.public_key(),
            ));
            self.client
                .pointer_update(&addressee.secret_key, target)
                .await?;
        }
        Ok(packages.iter().map(|p| p.0.clone()).collect())
    }

    pub async fn package_open(
        &self,
        package: &Package,
        secret_key: SecretKey,
    ) -> Result<Package, PostemError> {
        // Maybe in the event of the error from Cipertext::from_bytes should also return CannotDecrypt
        // !!! Needs testing as not sure this is correctly converting back to orginal data map
        let data_map = match secret_key.decrypt(&Ciphertext::from_bytes(package.seal().value())?) {
            Some(decrypted_data) => DataMapChunk::from(Chunk::new(Bytes::from(decrypted_data))),
            None => return Err(PostemError::CannotDecrypt),
        };
        let payload = match self.client.data_get(&data_map).await {
            Ok(payload) => payload,
            Err(_) => return Err(PostemError::CannotGetPayload),
        };
        Ok(package.clone_with_new_payload(payload))
    }

    pub async fn packages_open(
        &self,
        packages: &Vec<Package>,
        secret_key: SecretKey,
    ) -> Result<Vec<Package>, PostemError> {
        let mut open_packages = vec![];
        for package in packages {
            match self.package_open(package, secret_key.clone()).await {
                Err(PostemError::CannotDecrypt | PostemError::CannotGetPayload) => (),
                Ok(open_package) => open_packages.push(open_package),
                Err(e) => return Err(e),
            }
        }
        Ok(open_packages)
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

    use super::*;
    use crate::client::ConnectionType;
    use autonomi::{Bytes, ChunkAddress, XorName};

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
        let payment_option = client.get_payment_option("")?;
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
    // Assumes run from freshly started local client
    async fn create_addressee() -> Result<(), PostemError> {
        let client = PostemClient::init(ConnectionType::Local).await?;
        let payment_option = client.get_payment_option("")?;
        let name = "my.address";
        // let estimate = client.addressee_cost(&name).await?;
        // eprintln!("Estimate: {:?}", estimate);
        let addressee = client
            .addressee_create(name, payment_option.clone(), None)
            .await?;
        let derived_name = PostemName::create(name).unwrap();
        assert_eq!(addressee.address, derived_name);
        assert_eq!(
            addressee.base.0.owner.to_hex(),
            derived_name.derive_key().unwrap().public_key().to_hex()
        );
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
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let addressee = client
            .addressee_create(
                "my.address2",
                payment_option.clone(),
                Some(secret_key.clone()),
            )
            .await
            .unwrap();
        assert_eq!(
            addressee.address,
            PostemName::create("my.address2").unwrap()
        );
        assert_eq!(addressee.secret_key.to_hex(), secret_key.to_hex());
        Ok(())
    }

    #[tokio::test]
    async fn addressee_set_last_received() {
        let mut client = PostemClient::init(ConnectionType::Local).await.unwrap();
        let payment_option = client.get_payment_option("").unwrap();
        let name = "my.address3";
        let mut addressee = client
            .addressee_create(name, payment_option.clone(), None)
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let last_recieved_pointing_to_sk = match client
            .client
            .pointer_get(&addressee.last_received)
            .await
            .unwrap()
            .target()
        {
            PointerTarget::GraphEntryAddress(graph_address) => graph_address.owner().to_hex(),
            _ => panic!("Target should be pointing to graph"),
        };
        assert_eq!(
            last_recieved_pointing_to_sk,
            addressee.base().graph_entry().owner.to_hex()
        );
        let graph_address = GraphEntryAddress::new(SecretKey::random().public_key());
        client
            .addressee_set_last_received(&mut addressee, graph_address)
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let last_recieved_pointing_to_sk = match client
            .client
            .pointer_get(&addressee.last_received)
            .await
            .unwrap()
            .target()
        {
            PointerTarget::GraphEntryAddress(graph_address) => graph_address.owner().to_hex(),
            _ => panic!("Target should be pointing to graph"),
        };
        assert_eq!(last_recieved_pointing_to_sk, graph_address.to_hex());
    }

    #[tokio::test]
    async fn addressee_get_packages() {
        let mut client = PostemClient::init(ConnectionType::Local).await.unwrap();
        let payment_option = client.get_payment_option("").unwrap();
        let name = SecretKey::random().to_hex();
        let mut addressee = client
            .addressee_create(&name, payment_option.clone(), None)
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let packages = client
            .addressee_get_packages(&mut addressee, true)
            .await
            .unwrap();
        assert!(packages.is_empty());
        let message = Bytes::from("Hello");
        let (package, _attos) = client
            .package_post(addressee.address(), message.clone(), payment_option.clone())
            .await
            .unwrap();
        eprintln!("package pk: {}", package.address().owner.to_hex());
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let got_graph = client
            .client
            .graph_entry_get(&package.address().address())
            .await
            .unwrap();
        eprintln!("{:?}", got_graph);
        // let got_chunk = client
        //     .client
        //     .chunk_get(&ChunkAddress::new(XorName::from_content(
        //         Bytes::copy_from_slice(got_graph.descendants.first().unwrap().1),
        //     )))
        //     .await
        //     .unwrap();
        // eprintln!("{:?}", got_chunk.value());
        let packages = client
            .addressee_get_packages(&mut addressee, true)
            .await
            .unwrap();
        assert_eq!(packages.len(), 1);
    }

    #[tokio::test]
    async fn addressee_inspect_packages() {
        let mut client = PostemClient::init(ConnectionType::Local).await.unwrap();
        let payment_option = client.get_payment_option("").unwrap();
        let name = "my.address4";
        let mut addressee = client
            .addressee_create(name, payment_option.clone(), None)
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let last_recieved_pointing_to_sk = match client
            .client
            .pointer_get(&addressee.last_received)
            .await
            .unwrap()
            .target()
        {
            PointerTarget::GraphEntryAddress(graph_address) => graph_address.owner().to_hex(),
            _ => panic!("Target should be pointing to graph"),
        };
        assert_eq!(
            last_recieved_pointing_to_sk,
            addressee.base().graph_entry().owner.to_hex()
        );
        let packages = client
            .addressee_get_packages(&mut addressee, true)
            .await
            .unwrap();
        assert!(packages.is_empty());
        let items = client
            .addressee_inspect_packages(&mut addressee, &packages)
            .await
            .unwrap();
        assert!(items.is_empty());
        let message = Bytes::from("Hello");
        let (_package, _attos) = client
            .package_post(addressee.address(), message.clone(), payment_option.clone())
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let packages = client
            .addressee_get_packages(&mut addressee, true)
            .await
            .unwrap();
        assert!(!packages.is_empty());
        let items = client
            .addressee_inspect_packages(&mut addressee, &packages)
            .await
            .unwrap();
        assert!(!items.is_empty());
        assert_eq!(message, items.first().unwrap().payload().unwrap());
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
