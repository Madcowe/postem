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

pub mod addressee;
pub mod client;
pub mod door_mat;
pub mod error;
pub mod package;
pub mod route;

pub use crate::addressee::Addressee;
pub use crate::client::{ConnectionType, PostemClient};
pub use crate::door_mat::DoorMat;
pub use crate::error::PostemError;
pub use crate::package::Package;
pub use crate::route::Route;
