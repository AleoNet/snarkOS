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

use snarkos_errors::algorithms::PRFError;
use snarkos_models::algorithms::PRF;

use blake2::Blake2s as blake2s;
use digest::Digest;

#[derive(Clone)]
pub struct Blake2s;

impl PRF for Blake2s {
    type Input = [u8; 32];
    type Output = [u8; 32];
    type Seed = [u8; 32];

    fn evaluate(seed: &Self::Seed, input: &Self::Input) -> Result<Self::Output, PRFError> {
        let eval_time = start_timer!(|| "Blake2s::Eval");
        let mut h = blake2s::new();
        h.input(seed.as_ref());
        h.input(input.as_ref());

        let mut result = [0u8; 32];
        result.copy_from_slice(&h.result());
        end_timer!(eval_time);
        Ok(result)
    }
}
