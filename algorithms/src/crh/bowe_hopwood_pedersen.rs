use crate::crh::{bytes_to_bits, PedersenCRH, PedersenCRHParameters, PedersenSize};
use snarkos_errors::{algorithms::CRHError, curves::ConstraintFieldError};
use snarkos_models::{
    algorithms::CRH,
    curves::{to_field_vec::ToConstraintField, Field, Group, PrimeField},
};
use snarkos_utilities::biginteger::biginteger::BigInteger;

use rand::Rng;

#[cfg(feature = "pedersen-parallel")]
use rayon::prelude::*;

/// Returns an iterator over `chunk_size` elements of the slice at a
/// time.
macro_rules! cfg_chunks {
    ($e: expr, $size: expr) => {{
        #[cfg(feature = "pedersen-parallel")]
        let result = $e.par_chunks($size);

        #[cfg(not(feature = "pedersen-parallel"))]
        let result = $e.chunks($size);

        result
    }};
}

pub const CHUNK_SIZE: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BoweHopwoodPedersenCRH<G: Group, S: PedersenSize> {
    pub parameters: PedersenCRHParameters<G, S>,
}

impl<G: Group, S: PedersenSize> BoweHopwoodPedersenCRH<G, S> {
    pub fn create_generators<R: Rng>(rng: &mut R) -> Vec<Vec<G>> {
        let mut generators = Vec::new();
        for _ in 0..S::NUM_WINDOWS {
            let mut generators_for_segment = Vec::new();
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
            let mut range = F::BigInt::from(2_u64);
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
            W::NUM_WINDOWS,
            W::WINDOW_SIZE,
            W::WINDOW_SIZE * W::NUM_WINDOWS * CHUNK_SIZE
        ));
        let bases = Self::create_generators(rng);
        end_timer!(time);

        let parameters = Self::Parameters::from(bases);
        Self { parameters }
    }

    fn hash(&self, input: &[u8]) -> Result<Self::Output, CRHError> {
        let eval_time = start_timer!(|| "BoweHopwoodPedersenCRH::Eval");

        if (input.len() * 8) > S::WINDOW_SIZE * S::NUM_WINDOWS * CHUNK_SIZE {
            panic!(
                "incorrect input length {:?} for window params {:?}x{:?}x{}",
                input.len(),
                S::WINDOW_SIZE,
                S::NUM_WINDOWS,
                CHUNK_SIZE,
            );
        }

        let mut padded_input = Vec::with_capacity(input.len());
        let input = bytes_to_bits(input);
        // Pad the input if it is not the current length.
        padded_input.extend_from_slice(&input);
        if input.len() % CHUNK_SIZE != 0 {
            let current_length = input.len();
            for _ in 0..(CHUNK_SIZE - current_length % CHUNK_SIZE) {
                padded_input.push(false);
            }
        }

        assert_eq!(padded_input.len() % CHUNK_SIZE, 0);

        assert_eq!(
            self.parameters.bases.len(),
            S::NUM_WINDOWS,
            "Incorrect pp of size {:?} for window params {:?}x{:?}x{}",
            self.parameters.bases.len(),
            S::WINDOW_SIZE,
            S::NUM_WINDOWS,
            CHUNK_SIZE,
        );
        for bases in self.parameters.bases.iter() {
            assert_eq!(bases.len(), S::WINDOW_SIZE);
        }
        assert_eq!(CHUNK_SIZE, 3);

        // Compute sum of h_i^{sum of
        // (1-2*c_{i,j,2})*(1+c_{i,j,0}+2*c_{i,j,1})*2^{4*(j-1)} for all j in segment}
        // for all i. Described in section 5.4.1.7 in the Zcash protocol
        // specification.

        let result = cfg_chunks!(padded_input, S::WINDOW_SIZE * CHUNK_SIZE)
            .zip(&self.parameters.bases)
            .map(|(segment_bits, segment_generators)| {
                cfg_chunks!(segment_bits, CHUNK_SIZE)
                    .zip(segment_generators)
                    .map(|(chunk_bits, generator)| {
                        let mut encoded = generator.clone();
                        if chunk_bits[0] {
                            encoded = encoded + generator;
                        }
                        if chunk_bits[1] {
                            encoded += &generator.double();
                        }
                        if chunk_bits[2] {
                            encoded = encoded.neg();
                        }
                        encoded
                    })
                    .reduce(G::zero, |a, b| a + &b)
            })
            .reduce(G::zero, |a, b| a + &b);

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

//#[cfg(test)]
//mod test {
//    use crate::{
//        crh::{bowe_hopwood::BoweHopwoodPedersenCRH, pedersen::PedersenWindow},
//        FixedLengthCRH,
//    };
//    use algebra::{ed_on_bls12_381::EdwardsProjective, test_rng};
//
//    #[test]
//    fn test_simple_bh() {
//        #[derive(Clone)]
//        struct TestWindow {}
//        impl PedersenWindow for TestWindow {
//            const WINDOW_SIZE: usize = 63;
//            const NUM_WINDOWS: usize = 8;
//        }
//
//        let rng = &mut test_rng();
//        let params =
//            <BoweHopwoodPedersenCRH<EdwardsProjective, TestWindow> as FixedLengthCRH>::setup(rng)
//                .unwrap();
//        <BoweHopwoodPedersenCRH<EdwardsProjective, TestWindow> as FixedLengthCRH>::evaluate(
//            &params,
//            &[1, 2, 3],
//        )
//            .unwrap();
//    }
//}
