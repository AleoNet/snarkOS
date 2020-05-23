use crate::{commitment::PedersenCommitment, crh::PedersenSize};
use snarkos_curves::edwards_bls12::EdwardsProjective;
use snarkos_models::storage::Storage;
use snarkvm_models::algorithms::CommitmentScheme;
use snarkvm_utilities::{
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

const TEST_COMMITMENT_PARAMETERS_PATH: &str = "./pedersen_commitment.params";

#[test]
fn commitment_parameter_serialization() {
    let rng = &mut thread_rng();

    let commitment = TestCommitment::setup(rng);

    let commitment_bytes = to_bytes![commitment].unwrap();
    let recovered_commitment: TestCommitment = FromBytes::read(&commitment_bytes[..]).unwrap();

    assert_eq!(commitment, recovered_commitment);
}

#[test]
fn commitment_parameter_storage() {
    let rng = &mut thread_rng();
    let mut path = std::env::temp_dir();
    path.push(TEST_COMMITMENT_PARAMETERS_PATH);

    let commitment = TestCommitment::setup(rng);
    commitment.store(&path).unwrap();

    let recovered_commitment = TestCommitment::load(&path).unwrap();

    assert_eq!(commitment, recovered_commitment);

    std::fs::remove_file(&path).unwrap();
}
