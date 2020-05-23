use crate::{commitment::PedersenCommitment, crh::PedersenSize};
use snarkos_curves::edwards_bls12::EdwardsProjective;
use snarkos_models::algorithms::CommitmentScheme;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use rand::thread_rng;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Size;

impl PedersenSize for Size {
    const NUM_WINDOWS: usize = 8;
    const WINDOW_SIZE: usize = 4;
}

type TestCommitment = PedersenCommitment<EdwardsProjective, Size>;

#[test]
fn commitment_parameter_serialization() {
    let rng = &mut thread_rng();

    let commitment = TestCommitment::setup(rng);

    let commitment_bytes = to_bytes![commitment].unwrap();
    let recovered_commitment: TestCommitment = FromBytes::read(&commitment_bytes[..]).unwrap();

    assert_eq!(commitment, recovered_commitment);
}
