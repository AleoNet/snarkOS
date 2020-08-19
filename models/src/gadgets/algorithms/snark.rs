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

use crate::{
    algorithms::SNARK,
    curves::Field,
    gadgets::{
        r1cs::ConstraintSystem,
        utilities::{
            alloc::{AllocBytesGadget, AllocGadget},
            ToBitsGadget,
            ToBytesGadget,
        },
    },
};
use snarkos_errors::gadgets::SynthesisError;

pub trait SNARKVerifierGadget<N: SNARK, F: Field> {
    type VerificationKeyGadget: AllocGadget<N::VerificationParameters, F>
        + AllocBytesGadget<Vec<u8>, F>
        + ToBytesGadget<F>;
    type ProofGadget: AllocGadget<N::Proof, F> + AllocBytesGadget<Vec<u8>, F>;

    fn check_verify<'a, CS: ConstraintSystem<F>, I: Iterator<Item = &'a T>, T: 'a + ToBitsGadget<F> + ?Sized>(
        cs: CS,
        verification_key: &Self::VerificationKeyGadget,
        input: I,
        proof: &Self::ProofGadget,
    ) -> Result<(), SynthesisError>;
}
