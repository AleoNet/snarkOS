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

use snarkos_errors::algorithms::SNARKError;
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use rand::Rng;
use std::fmt::Debug;

pub trait SNARK {
    type AssignedCircuit;
    type Circuit;
    type Proof: Clone + Debug + ToBytes + FromBytes;
    type PreparedVerificationParameters: Clone + From<Self::VerificationParameters> + From<Self::ProvingParameters>;
    type ProvingParameters: Clone + ToBytes + FromBytes;
    type VerificationParameters: Clone
        + ToBytes
        + FromBytes
        + From<Self::PreparedVerificationParameters>
        + From<Self::ProvingParameters>;
    type VerifierInput: ?Sized;

    fn setup<R: Rng>(
        circuit: &Self::Circuit,
        rng: &mut R,
    ) -> Result<(Self::ProvingParameters, Self::PreparedVerificationParameters), SNARKError>;

    fn prove<R: Rng>(
        parameter: &Self::ProvingParameters,
        input_and_witness: &Self::AssignedCircuit,
        rng: &mut R,
    ) -> Result<Self::Proof, SNARKError>;

    fn verify(
        verifier_key: &Self::PreparedVerificationParameters,
        input: &Self::VerifierInput,
        proof: &Self::Proof,
    ) -> Result<bool, SNARKError>;
}
