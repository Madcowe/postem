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
use postem::{Addressee, DoorMat, Package, PostemClient, addressee::PostemName};

enum AppState {
    None,
    CreateAddresseei(CreateAddresseeState),
    SendPackagee(SendPackageStatus),
    ViewDoormat,
    ViewPackage,
}

enum CreateAddresseeState {
    InputAddresseeName,
    InputFundingWalled,
}

enum SendPackageStatus {
    InputRecipients,
    InputMessage,
    InputFundingWallet,
}

struct App {
    app_state: AppState,
    client: PostemClient,
    doormat: Option<DoorMat>,
    recipients: Vec<PostemName>,
    package: Option<Package>,
}
