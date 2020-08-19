// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::algorithms::{CommitmentScheme, EncryptionScheme, SignatureScheme};
use snarkos_errors::objects::AccountError;

use rand::Rng;

pub trait AccountScheme: Sized {
    type AccountAddress: Default;
    type AccountPrivateKey;
    type CommitmentScheme: CommitmentScheme;
    type EncryptionScheme: EncryptionScheme;
    type SignatureScheme: SignatureScheme;

    fn new<R: Rng>(
        signature_parameters: &Self::SignatureScheme,
        commitment_parameters: &Self::CommitmentScheme,
        encryption_parameters: &Self::EncryptionScheme,
        rng: &mut R,
    ) -> Result<Self, AccountError>;
}
