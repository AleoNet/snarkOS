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
    bls12_377::{
        g1::Bls12_377G1Parameters,
        g2::Bls12_377G2Parameters,
        Fq,
        Fq12,
        Fq12Parameters,
        Fq2Parameters,
        Fq6Parameters,
    },
    templates::bls12::{
        Bls12,
        Bls12Parameters,
        G1Affine as Bls12G1Affine,
        G1Prepared,
        G1Projective as Bls12G1Projective,
        G2Affine as Bls12G2Affine,
        G2Prepared,
        G2Projective as Bls12G2Projective,
        TwistType,
    },
};
use snarkos_models::curves::{PairingCurve, PairingEngine};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bls12_377Parameters;

impl Bls12Parameters for Bls12_377Parameters {
    type Fp = Fq;
    type Fp12Params = Fq12Parameters;
    type Fp2Params = Fq2Parameters;
    type Fp6Params = Fq6Parameters;
    type G1Parameters = Bls12_377G1Parameters;
    type G2Parameters = Bls12_377G2Parameters;

    const TWIST_TYPE: TwistType = TwistType::D;
    const X: &'static [u64] = &[0x8508c00000000001];
    /// `x` is positive.
    const X_IS_NEGATIVE: bool = false;
}

pub type Bls12_377 = Bls12<Bls12_377Parameters>;

pub type G2Affine = Bls12G2Affine<Bls12_377Parameters>;
pub type G2Projective = Bls12G2Projective<Bls12_377Parameters>;

pub type G1Affine = Bls12G1Affine<Bls12_377Parameters>;
pub type G1Projective = Bls12G1Projective<Bls12_377Parameters>;

impl PairingCurve for G1Affine {
    type Engine = Bls12_377;
    type PairWith = G2Affine;
    type PairingResult = Fq12;
    type Prepared = G1Prepared<Bls12_377Parameters>;

    fn prepare(&self) -> Self::Prepared {
        Self::Prepared::from_affine(*self)
    }

    fn pairing_with(&self, other: &Self::PairWith) -> Self::PairingResult {
        Bls12_377::pairing(*self, *other)
    }
}

impl PairingCurve for G2Affine {
    type Engine = Bls12_377;
    type PairWith = G1Affine;
    type PairingResult = Fq12;
    type Prepared = G2Prepared<Bls12_377Parameters>;

    fn prepare(&self) -> Self::Prepared {
        Self::Prepared::from_affine(*self)
    }

    fn pairing_with(&self, other: &Self::PairWith) -> Self::PairingResult {
        Bls12_377::pairing(*other, *self)
    }
}
