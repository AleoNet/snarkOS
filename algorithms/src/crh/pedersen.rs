use crate::crh::{PedersenCRHParameters, PedersenSize};
use snarkos_models::{
    curves::{to_field_vec::ToConstraintField, Field, Group},
    storage::Storage,
};
use snarkvm_errors::{algorithms::CRHError, curves::ConstraintFieldError};
use snarkvm_models::algorithms::CRH;
use snarkvm_utilities::bytes::{FromBytes, ToBytes};

use rand::Rng;
use rayon::prelude::*;
use std::{
    io::{Read, Result as IoResult, Write},
    path::PathBuf,
};

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

impl<G: Group, S: PedersenSize> ToBytes for PedersenCRH<G, S> {
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.parameters.write(&mut writer)
    }
}

impl<G: Group, S: PedersenSize> FromBytes for PedersenCRH<G, S> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let parameters: PedersenCRHParameters<G, S> = FromBytes::read(&mut reader)?;

        Ok(Self { parameters })
    }
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

    fn hash(&self, input: &[u8]) -> Result<Self::Output, CRHError> {
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

    fn parameters(&self) -> &Self::Parameters {
        &self.parameters
    }
}

impl<G: Group, S: PedersenSize> Storage for PedersenCRH<G, S> {
    /// Store the Pedersen CRH parameters to a file at the given path.
    fn store(&self, path: &PathBuf) -> IoResult<()> {
        self.parameters.store(path)?;

        Ok(())
    }

    /// Load the Pedersen CRH parameters from a file at the given path.
    fn load(path: &PathBuf) -> IoResult<Self> {
        let parameters = PedersenCRHParameters::<G, S>::load(path)?;

        Ok(Self { parameters })
    }
}

impl<G: Group, S: PedersenSize> From<PedersenCRHParameters<G, S>> for PedersenCRH<G, S> {
    fn from(parameters: PedersenCRHParameters<G, S>) -> Self {
        Self { parameters }
    }
}

impl<F: Field, G: Group + ToConstraintField<F>, S: PedersenSize> ToConstraintField<F> for PedersenCRH<G, S> {
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, ConstraintFieldError> {
        self.parameters.to_field_elements()
    }
}
