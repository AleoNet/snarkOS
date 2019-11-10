use crate::crh::{PedersenCRHParameters, PedersenSize};
use snarkos_errors::algorithms::{CryptoError, Error};
use snarkos_models::{
    algorithms::CRH,
    curves::{AffineCurve, Group, ProjectiveCurve},
};

use rand::Rng;
use rayon::prelude::*;

pub fn bytes_to_bits(bytes: &[u8]) -> Vec<bool> {
    let mut bits = Vec::with_capacity(bytes.len() * 8);
    for byte in bytes {
        for i in 0..8 {
            let bit = (*byte >> i) & 1;
            bits.push(bit == 1)
        }
    }
    bits
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PedersenCRH<G: Group, S: PedersenSize> {
    pub parameters: PedersenCRHParameters<G, S>,
}

impl<G: Group, S: PedersenSize> CRH for PedersenCRH<G, S> {
    type Output = G;
    type Parameters = PedersenCRHParameters<G, S>;

    const INPUT_SIZE_BITS: usize = S::WINDOW_SIZE * S::NUM_WINDOWS;

    fn setup<R: Rng>(rng: &mut R) -> Self {
        Self {
            parameters: PedersenCRHParameters::new(rng),
        }
    }

    fn hash(&self, input: &[u8]) -> Result<Self::Output, Error> {
        if (input.len() * 8) > S::WINDOW_SIZE * S::NUM_WINDOWS {
            // TODO (howardwu): Return a CRHError.
            panic!(
                "incorrect input length {:?} for window params {:?}x{:?}",
                input.len(),
                S::WINDOW_SIZE,
                S::NUM_WINDOWS
            );
        }

        let mut padded_input = vec![];
        let mut input = input;
        // Pad the input if it is not the current length.
        if (input.len() * 8) < S::WINDOW_SIZE * S::NUM_WINDOWS {
            padded_input.extend_from_slice(input);
            for _ in input.len()..((S::WINDOW_SIZE * S::NUM_WINDOWS) / 8) {
                padded_input.push(0u8);
            }
            input = padded_input.as_slice();
        }

        // TODO (howardwu): Return a CRHError.
        assert_eq!(
            self.parameters.bases.len(),
            S::NUM_WINDOWS,
            "Incorrect pp of size {:?}x{:?} for window params {:?}x{:?}",
            self.parameters.bases[0].len(),
            self.parameters.bases.len(),
            S::WINDOW_SIZE,
            S::NUM_WINDOWS
        );

        // Compute sum of h_i^{m_i} for all i.
        let result = bytes_to_bits(input)
            .par_chunks(S::WINDOW_SIZE)
            .zip(&self.parameters.bases)
            .map(|(bits, powers)| {
                let mut encoded = G::zero();
                for (bit, base) in bits.iter().zip(powers.iter()) {
                    if *bit {
                        encoded += base;
                    }
                }
                encoded
            })
            .reduce(G::zero, |a, b| a + &b);

        Ok(result)
    }
}

impl<G: Group + ProjectiveCurve, S: PedersenSize> PedersenCRH<G, S> {
    /// Returns the affine x-coordinate of a given collision-resistant hash output.
    fn compress(output: G) -> Result<<G::Affine as AffineCurve>::BaseField, CryptoError> {
        let affine = output.into_affine();
        debug_assert!(affine.is_in_correct_subgroup_assuming_on_curve());
        Ok(affine.to_x_coordinate())
    }
}

impl<G: Group, S: PedersenSize> From<PedersenCRHParameters<G, S>> for PedersenCRH<G, S> {
    fn from(parameters: PedersenCRHParameters<G, S>) -> Self {
        Self { parameters }
    }
}
