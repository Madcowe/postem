/*
Copyright (C) 2026 Postem

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

use crate::App;
use postem::{Addressee, addressee::PostemName};
use serde::{Deserialize, Serialize};
use std::{fs, ops::Index};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Account {
    pub name: String,
    pub secret_key: String,
}
impl From<Addressee> for Account {
    fn from(addressee: Addressee) -> Self {
        Account {
            name: addressee.address().name(),
            secret_key: addressee.secret_key().to_hex(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Accounts {
    pub accounts: Vec<Account>,
}
impl Accounts {
    pub fn new() -> Accounts {
        Accounts { accounts: vec![] }
    }

    pub fn load_file(path: &str) -> Option<Accounts> {
        if let Ok(account_string) = fs::read_to_string(path) {
            if let Ok(account) = toml::from_str(&account_string) {
                return Some(account);
            } else {
                return None;
            }
        } else {
            return None;
        }
    }

    /// Returns true if successful
    pub fn save_file(&self, path: &str) -> bool {
        if let Ok(account_string) = toml::to_string(&self) {
            let Ok(()) = fs::write(path, &account_string) else {
                return false;
            };
        } else {
            return false;
        }
        true
    }

    pub fn add(&mut self, account: Account) {
        self.accounts.push(account);
    }

    pub fn size(&self) -> usize {
        self.accounts.len()
    }

    pub fn get_account(&self, index: usize) -> Option<Account> {
        self.accounts.get(index).cloned()
    }

    pub fn as_table(&self) -> Vec<String> {
        let mut v = vec![];
        for account in self.accounts.iter() {
            v.push(account.name.clone())
        }
        v
    }
}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]
    pub fn test_accounts_add() {
        let path = "test.toml";
        let mut accounts = Accounts::new();
        let account = Account {
            name: "Test".to_string(),
            secret_key: "TEST".to_string(),
        };
        accounts.add(account);
        assert!(accounts.save_file(path));
    }
}
