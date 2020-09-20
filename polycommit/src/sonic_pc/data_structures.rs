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

use crate::{impl_bytes, kzg10, PCCommitterKey, PCVerifierKey, Vec};
use snarkos_errors::serialization::SerializationError;
use snarkos_models::curves::{PairingCurve, PairingEngine};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    error,
    serialize::*,
};
use std::collections::BTreeMap;

/// `UniversalParams` are the universal parameters for the KZG10 scheme.
pub type UniversalParams<E> = kzg10::UniversalParams<E>;

/// `Randomness` is the randomness for the KZG10 scheme.
pub type Randomness<E> = kzg10::Randomness<E>;

/// `Commitment` is the commitment for the KZG10 scheme.
pub type Commitment<E> = kzg10::Commitment<E>;

/// `ComitterKey` is used to commit to, and create evaluation proofs for, a given
/// polynomial.
#[derive(Derivative)]
#[derivative(Default(bound = ""), Hash(bound = ""), Clone(bound = ""), Debug(bound = ""))]
#[derive(CanonicalSerialize, CanonicalDeserialize)]
pub struct CommitterKey<E: PairingEngine> {
    /// The key used to commit to polynomials.
    pub powers_of_g: Vec<E::G1Affine>,

    /// The key used to commit to hiding polynomials.
    pub powers_of_gamma_g: Vec<E::G1Affine>,

    /// The powers used to commit to shifted polynomials.
    /// This is `None` if `self` does not support enforcing any degree bounds.
    pub shifted_powers_of_g: Option<Vec<E::G1Affine>>,

    /// The powers used to commit to shifted hiding polynomials.
    /// This is `None` if `self` does not support enforcing any degree bounds.
    pub shifted_powers_of_gamma_g: Option<BTreeMap<usize, Vec<E::G1Affine>>>,

    /// The degree bounds that are supported by `self`.
    /// Sorted in ascending order from smallest bound to largest bound.
    /// This is `None` if `self` does not support enforcing any degree bounds.
    pub enforced_degree_bounds: Option<Vec<usize>>,

    /// The maximum degree supported by the `UniversalParams` from which `self` was derived
    pub max_degree: usize,
}
impl_bytes!(CommitterKey);

impl<E: PairingEngine> CommitterKey<E> {
    /// Obtain powers for the underlying KZG10 construction
    pub fn powers(&self) -> kzg10::Powers<E> {
        kzg10::Powers {
            powers_of_g: self.powers_of_g.as_slice().into(),
            powers_of_gamma_g: self.powers_of_gamma_g.as_slice().into(),
        }
    }

    /// Obtain powers for committing to shifted polynomials.
    pub fn shifted_powers(&self, degree_bound: impl Into<Option<usize>>) -> Option<kzg10::Powers<E>> {
        match (&self.shifted_powers_of_g, &self.shifted_powers_of_gamma_g) {
            (Some(shifted_powers_of_g), Some(shifted_powers_of_gamma_g)) => {
                let max_bound = self.enforced_degree_bounds.as_ref().unwrap().last().unwrap();
                let (bound, powers_range) = if let Some(degree_bound) = degree_bound.into() {
                    assert!(self.enforced_degree_bounds.as_ref().unwrap().contains(&degree_bound));
                    (degree_bound, (max_bound - degree_bound)..)
                } else {
                    (*max_bound, 0..)
                };

                let ck = kzg10::Powers {
                    powers_of_g: shifted_powers_of_g[powers_range].into(),
                    powers_of_gamma_g: shifted_powers_of_gamma_g[&bound].clone().into(),
                };

                Some(ck)
            }

            (_, _) => None,
        }
    }
}

impl<E: PairingEngine> PCCommitterKey for CommitterKey<E> {
    fn max_degree(&self) -> usize {
        self.max_degree
    }

    fn supported_degree(&self) -> usize {
        self.powers_of_g.len() - 1
    }
}

/// `VerifierKey` is used to check evaluation proofs for a given commitment.
#[derive(Derivative)]
#[derivative(Default(bound = ""), Clone(bound = ""), Debug(bound = ""))]
#[derive(CanonicalSerialize, CanonicalDeserialize)]
pub struct VerifierKey<E: PairingEngine> {
    /// The generator of G1.
    pub g: E::G1Affine,

    /// The generator of G1 that is used for making a commitment hiding.
    pub gamma_g: E::G1Affine,

    /// The generator of G2.
    pub h: E::G2Affine,

    /// \beta times the generator of G2.
    pub beta_h: E::G2Affine,

    /// The generator of G2, prepared for use in pairings.
    pub prepared_h: <E::G2Affine as PairingCurve>::Prepared,

    /// The \beta times the generator of G2, prepared for use in pairings.
    pub prepared_beta_h: <E::G2Affine as PairingCurve>::Prepared,

    /// Pairs a degree_bound with its corresponding G2 element, which has been prepared for use in pairings.
    /// Each pair is in the form `(degree_bound, \beta^{degree_bound - max_degree} h),` where `h` is the generator of G2 above
    pub degree_bounds_and_prepared_neg_powers_of_h: Option<Vec<(usize, <E::G2Affine as PairingCurve>::Prepared)>>,

    /// The maximum degree supported by the trimmed parameters that `self` is
    /// a part of.
    pub supported_degree: usize,

    /// The maximum degree supported by the `UniversalParams` `self` was derived
    /// from.
    pub max_degree: usize,
}
impl_bytes!(VerifierKey);

impl<E: PairingEngine> VerifierKey<E> {
    /// Find the appropriate shift for the degree bound.
    pub fn get_shift_power(&self, degree_bound: usize) -> Option<<E::G2Affine as PairingCurve>::Prepared> {
        self.degree_bounds_and_prepared_neg_powers_of_h.as_ref().and_then(|v| {
            v.binary_search_by(|(d, _)| d.cmp(&degree_bound))
                .ok()
                .map(|i| v[i].1.clone())
        })
    }
}

impl<E: PairingEngine> PCVerifierKey for VerifierKey<E> {
    fn max_degree(&self) -> usize {
        self.max_degree
    }

    fn supported_degree(&self) -> usize {
        self.supported_degree
    }
}

/// Evaluation proof at a query set.
#[derive(Derivative)]
#[derivative(
    Default(bound = ""),
    Hash(bound = ""),
    Clone(bound = ""),
    Debug(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = "")
)]
pub struct BatchProof<E: PairingEngine>(pub(crate) Vec<kzg10::Proof<E>>);
