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
use autonomi::Chunk;
use autonomi::client::GetError;
use autonomi::client::PutError;
use autonomi::client::quote::CostError;
use autonomi::graph::GraphError;
use autonomi::pointer::PointerError;
use blsttc::SecretKey;

#[derive(Debug, thiserror::Error, PartialEq, Clone)]
pub enum PostemError {
    #[error("Could nor initiate autonomi client")]
    ClientConnectionError,
    #[error("Reqested name is blank. Note full stops are not counted as delimnate domains")]
    BlankName,
    #[error("Requested name contains the following invalid character: {0}")]
    NameContainsInvalidCharatcers(char),
    #[error("Graph entry error: {0}")]
    GraphEntryError(String),
    #[error("An addressee already exists with name: {0}\nPlease choose another one.")]
    NameAlreadyExists(String, Option<SecretKey>),
    #[error("Pointer error: {0}")]
    PointerEntryError(String),
    #[error("Could not get funded wallet with key: |{0}| {1}")]
    FailedToGetWallet(String, String),
    #[error("{0}")]
    BLSError(String),
    #[error("{0}")]
    CostError(String),
    #[error("{0}")]
    PutError(String),
    #[error("Graph entry is not a valid postem address base")]
    NotValidPostemBase(),
    #[error("Tried to retrieve package from free location")]
    EmptyLocation,
    #[error("Tried to retrieve package from blocked location")]
    BlockedLocation,
    #[error("Package invalid as has no retrievable seal")]
    MissingSeal,
    #[error("Get error: {0}")]
    GetError(String),
    #[error("Cannot decrypt package seal")]
    CannotDecrypt,
    #[error("Cannot not get payload from decrypted seal")]
    CannotGetPayload,
    #[error("Last received location is not valid")]
    InvalidLastReceived,
    #[error("Postem address does not exist: {0}")]
    InvalidAddress(String),
    #[error(
        "Storage location repeatdily being used before pacakge is fully posted, chunk address of seal included for resuse"
    )]
    ChasingItsTail(Chunk),
    #[error("Antnet request never returned")]
    AntnetTimeOut,
    #[error("Package payload too small must be at least 3 bytes to send")]
    PayloadTooSmall,
}
// this should'nt automatically convert as non addresses base function could return the graph error
impl From<GraphError> for PostemError {
    fn from(e: GraphError) -> Self {
        // match e {
        //     GraphError::AlreadyExists(a) => PostemError::NameAlreadyExists(a.to_hex(), None),
        //     _ => {
        let message = format!("{e}");
        PostemError::GraphEntryError(message)
        //     }
        // }
    }
}

impl From<PointerError> for PostemError {
    fn from(e: PointerError) -> Self {
        let message = format!("{e}");
        PostemError::PointerEntryError(message)
    }
}

impl From<GetError> for PostemError {
    fn from(e: GetError) -> Self {
        let message = format!("{e}");
        PostemError::GetError(message)
    }
}
impl From<blsttc::Error> for PostemError {
    fn from(e: blsttc::Error) -> Self {
        let message = format!("{e}");
        PostemError::BLSError(message)
    }
}

impl From<CostError> for PostemError {
    fn from(e: CostError) -> Self {
        let message = format!("{e}");
        PostemError::CostError(message)
    }
}

impl From<PutError> for PostemError {
    fn from(e: PutError) -> Self {
        let message = format!("{e}");
        PostemError::PutError(message)
    }
}
