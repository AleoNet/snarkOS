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

use crate::crh::{PedersenCRH, PedersenCRHParameters, PedersenSize};
use snarkos_errors::{algorithms::CRHError, curves::ConstraintFieldError};
use snarkos_models::{
    algorithms::CRH,
    curves::{to_field_vec::ToConstraintField, Field, Group, PrimeField},
};
use snarkos_utilities::{biginteger::biginteger::BigInteger, bytes_to_bits};

use rand::Rng;

#[cfg(feature = "pedersen-parallel")]
use rayon::prelude::*;

/// Returns an iterator over `chunk_size` elements of the slice at a
/// time.
#[macro_export]
macro_rules! cfg_chunks {
    ($e: expr, $size: expr) => {{
        #[cfg(feature = "pedersen-parallel")]
        let result = $e.par_chunks($size);

        #[cfg(not(feature = "pedersen-parallel"))]
        let result = $e.chunks($size);

        result
    }};
}

/// Applies the reduce operation over an iterator.
#[macro_export]
macro_rules! cfg_reduce {
    ($e: expr, $default: expr, $op: expr) => {{
        #[cfg(feature = "pedersen-parallel")]
        let result = $e.reduce($default, $op);

        #[cfg(not(feature = "pedersen-parallel"))]
        let result = $e.fold($default(), $op);

        result
    }};
}

pub const BOWE_HOPWOOD_CHUNK_SIZE: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BoweHopwoodPedersenCRH<G: Group, S: PedersenSize> {
    pub parameters: PedersenCRHParameters<G, S>,
}

impl<G: Group, S: PedersenSize> BoweHopwoodPedersenCRH<G, S> {
    pub fn create_generators<R: Rng>(rng: &mut R) -> Vec<Vec<G>> {
        let mut generators = Vec::with_capacity(S::NUM_WINDOWS);
        for _ in 0..S::NUM_WINDOWS {
            let mut generators_for_segment = Vec::with_capacity(S::WINDOW_SIZE);
            let mut base = G::rand(rng);
            for _ in 0..S::WINDOW_SIZE {
                generators_for_segment.push(base);
                for _ in 0..4 {
                    base.double_in_place();
                }
            }
            generators.push(generators_for_segment);
        }
        generators
    }
}

impl<G: Group, S: PedersenSize> CRH for BoweHopwoodPedersenCRH<G, S> {
    type Output = G;
    type Parameters = PedersenCRHParameters<G, S>;

    const INPUT_SIZE_BITS: usize = PedersenCRH::<G, S>::INPUT_SIZE_BITS;

    fn setup<R: Rng>(rng: &mut R) -> Self {
        fn calculate_num_chunks_in_segment<F: PrimeField>() -> usize {
            let upper_limit = F::modulus_minus_one_div_two();
            let mut c = 0;
            let mut range = F::BigInteger::from(2_u64);
            while range < upper_limit {
                range.muln(4);
                c += 1;
            }

            c
        }

        let maximum_num_chunks_in_segment = calculate_num_chunks_in_segment::<G::ScalarField>();
        if S::WINDOW_SIZE > maximum_num_chunks_in_segment {
            panic!(
                "Bowe-Hopwood hash must have a window size resulting in scalars < (p-1)/2, \
                 maximum segment size is {}",
                maximum_num_chunks_in_segment
            );
        }

        let time = start_timer!(|| format!(
            "BoweHopwoodPedersenCRH::Setup: {} segments of {} 3-bit chunks; {{0,1}}^{{{}}} -> G",
            S::NUM_WINDOWS,
            S::WINDOW_SIZE,
            S::WINDOW_SIZE * S::NUM_WINDOWS * BOWE_HOPWOOD_CHUNK_SIZE
        ));
        let bases = Self::create_generators(rng);
        end_timer!(time);

        let parameters = Self::Parameters::from(bases);
        Self { parameters }
    }

    fn hash(&self, input: &[u8]) -> Result<Self::Output, CRHError> {
        let eval_time = start_timer!(|| "BoweHopwoodPedersenCRH::Eval");

        if (input.len() * 8) > S::WINDOW_SIZE * S::NUM_WINDOWS {
            return Err(CRHError::IncorrectInputLength(
                input.len(),
                S::WINDOW_SIZE,
                S::NUM_WINDOWS,
            ));
        }

        // Pad the input if it is not the current length.
        let mut input_bytes = input;
        let mut padded_input_bytes = vec![];
        if (input.len() * 8) < S::WINDOW_SIZE * S::NUM_WINDOWS {
            padded_input_bytes.extend_from_slice(input_bytes);
            for _ in input.len()..((S::WINDOW_SIZE * S::NUM_WINDOWS) / 8) {
                padded_input_bytes.push(0u8);
            }
            input_bytes = padded_input_bytes.as_slice();
        }

        let mut padded_input = Vec::with_capacity(input_bytes.len());
        let input = bytes_to_bits(input_bytes);
        // Pad the input if it is not the current length.
        padded_input.extend_from_slice(&input);
        if input.len() % BOWE_HOPWOOD_CHUNK_SIZE != 0 {
            let current_length = input.len();
            for _ in 0..(BOWE_HOPWOOD_CHUNK_SIZE - current_length % BOWE_HOPWOOD_CHUNK_SIZE) {
                padded_input.push(false);
            }
        }

        assert_eq!(padded_input.len() % BOWE_HOPWOOD_CHUNK_SIZE, 0);

        assert_eq!(
            self.parameters.bases.len(),
            S::NUM_WINDOWS,
            "Incorrect pp of size {:?} for window params {:?}x{:?}x{}",
            self.parameters.bases.len(),
            S::WINDOW_SIZE,
            S::NUM_WINDOWS,
            BOWE_HOPWOOD_CHUNK_SIZE,
        );
        for bases in self.parameters.bases.iter() {
            assert_eq!(bases.len(), S::WINDOW_SIZE);
        }
        assert_eq!(BOWE_HOPWOOD_CHUNK_SIZE, 3);

        // Compute sum of h_i^{sum of
        // (1-2*c_{i,j,2})*(1+c_{i,j,0}+2*c_{i,j,1})*2^{4*(j-1)} for all j in segment}
        // for all i. Described in section 5.4.1.7 in the Zcash protocol
        // specification.

        // TODO (howardwu): Are clever macros really better than repeating code for cfg?

        let result = cfg_reduce!(
            cfg_chunks!(padded_input, S::WINDOW_SIZE * BOWE_HOPWOOD_CHUNK_SIZE)
                .zip(&self.parameters.bases)
                .map(|(segment_bits, segment_generators)| {
                    cfg_reduce!(
                        cfg_chunks!(segment_bits, BOWE_HOPWOOD_CHUNK_SIZE)
                            .zip(segment_generators)
                            .map(|(chunk_bits, generator)| {
                                let mut encoded = *generator;
                                if chunk_bits[0] {
                                    encoded += generator;
                                }
                                if chunk_bits[1] {
                                    encoded += &generator.double();
                                }
                                if chunk_bits[2] {
                                    encoded = encoded.neg();
                                }
                                encoded
                            }),
                        G::zero,
                        |a, b| a + &b
                    )
                }),
            G::zero,
            |a, b| a + &b
        );

        end_timer!(eval_time);

        Ok(result)
    }

    fn parameters(&self) -> &Self::Parameters {
        &self.parameters
    }
}

impl<G: Group, S: PedersenSize> From<PedersenCRHParameters<G, S>> for BoweHopwoodPedersenCRH<G, S> {
    fn from(parameters: PedersenCRHParameters<G, S>) -> Self {
        Self { parameters }
    }
}

impl<F: Field, G: Group + ToConstraintField<F>, S: PedersenSize> ToConstraintField<F> for BoweHopwoodPedersenCRH<G, S> {
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, ConstraintFieldError> {
        self.parameters.to_field_elements()
    }
}
