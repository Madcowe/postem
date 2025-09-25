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
use autonomi::{
    Bytes, GraphEntry, GraphEntryAddress, Pointer, PointerAddress, PublicKey, SecretKey,
};

use crate::{client::PostemClient, error::PostemError};

/// Hex key of base (not) secret key of bored derive names
const POSTEM_DERIVED_KEY_BASE: &str =
    "00000000000000000000000000000000000000000000000000000003C2BABF81";

/// Characters that are not allowed in an addressee name
/// Currenty the most common deliminators so as too be able to differntiate mutiple addressees
const POSTEM_NAME_INVALID_CHARS: [char; 2] = [',', ';'];

/// The unique name of an addressee, full stops (.) deliminate domains.
#[derive(Debug, PartialEq)]
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
    pub async fn name_check_exists(&self, public_key: &PublicKey) -> Result<bool, PostemError> {
        Ok(self
            .client
            .graph_entry_check_existence(&GraphEntryAddress::new(public_key.clone()))
            .await?)
    }
}

/// The addressee which can receive packages. Consisting of the address where the base is located,
/// the base from which the location of pacakges sent to it can be derived, and last recieved a
/// pointer to the latest valid package recieved. Received in this context means the owner of the
/// address has followed the chain of pacakges from the base up to this point...there may well be
/// pacakges that have been sent since they last checked beyond this location.
// pub struct Addressee {
//     address: Name,
//     secret_key: SecretKey,
//     base: Base,
//     last_recieved: Pointer,
// }
// impl PostemClient {
//     async fn addressee_create(&self, )
// }
// impl Addressee {
//     fn create(name &str, payment_option: PaymentOption) -> Result<Self> {
//         let address = address_from_string(name);
//         /// check address available
//         let last_recieve = create_pointer else return Err(NameAlredyTaken());
//         let base = Base::create(address, last recieved);
//         // what if name is taken between last recieved being created
//         Ok(Addressee{
//             address,
//             base,
//             last_recieved,
//         })
//     }
// }

// /// derives as secrey key from the derive name base and name
// fn address_from_string(name &str) -> Result<SecretKey> {
// }

// pub struct Base(GraphEntry);
// the content field is the (address/public key) of a pointer to the last received package
// the key pair used to create said pointer is also use for encrypton
// impl Base {
//     fn create(address: SecretKey, last_recieved: Pointer ) -> Result<Self> {

//     }
// }

#[cfg(test)]
mod tests {

    use super::*;

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
    async fn name_check_exists() -> Result<(), PostemError> {
        let client = PostemClient::init(crate::client::ConnectionType::Local).await?;
        let name = PostemName::create("dom.dom")?;
        let secret_key = name.derive_key()?;
        let public_key = secret_key.public_key();
        let bool = client.name_check_exists(&public_key).await?;
        assert_eq!(bool, false);
        let target = PointerTarget::PointerAddress(PointerAddress::new(public_key));
        let payment_option = client.get_payment_option("").await?;
        client
            .client
            .pointer_create(&secret_key, target, payment_option)
            .await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let bool = client.name_check_exists(&public_key).await?;
        assert_eq!(bool, true);
        Ok(())
    }
}
