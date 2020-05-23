use crate::{
    commitment::{PedersenCommitment, PedersenCompressedCommitment},
    crh::PedersenSize,
};
use snarkos_curves::edwards_bls12::EdwardsProjective;
use snarkos_models::algorithms::CommitmentScheme;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Size;

impl PedersenSize for Size {
    const NUM_WINDOWS: usize = 8;
    const WINDOW_SIZE: usize = 4;
}

fn commitment_parameters_serialization<C: CommitmentScheme>() {
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);

    let commitment = C::new(rng);
    let commitment_parameters = commitment.parameters();

    let commitment_parameters_bytes = to_bytes![commitment_parameters].unwrap();
    let recovered_commitment_parameters: <C as CommitmentScheme>::Parameters =
        FromBytes::read(&commitment_parameters_bytes[..]).unwrap();

    assert_eq!(commitment_parameters, &recovered_commitment_parameters);
}

#[test]
fn pedersen_commitment_parameters_serialization() {
    commitment_parameters_serialization::<PedersenCommitment<EdwardsProjective, Size>>();
}

#[test]
fn pedersen_compressed_commitment_parameters_serialization() {
    commitment_parameters_serialization::<PedersenCompressedCommitment<EdwardsProjective, Size>>();
}
