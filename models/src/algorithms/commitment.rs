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

use snarkos_errors::algorithms::CommitmentError;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    rand::UniformRand,
};

use rand::Rng;
use std::{fmt::Debug, hash::Hash};

pub trait CommitmentScheme: Sized + Clone + From<<Self as CommitmentScheme>::Parameters> {
    type Output: Clone + Debug + Default + Eq + Hash + ToBytes + FromBytes;
    type Parameters: Clone + Debug + Eq + ToBytes + FromBytes;
    type Randomness: Clone + Debug + Default + Eq + UniformRand + ToBytes + FromBytes;

    fn setup<R: Rng>(r: &mut R) -> Self;

    fn commit(&self, input: &[u8], randomness: &Self::Randomness) -> Result<Self::Output, CommitmentError>;

    fn parameters(&self) -> &Self::Parameters;
}
