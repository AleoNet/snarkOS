use crate::crh::{PedersenCRH, PedersenSize};
use snarkos_curves::edwards_bls12::EdwardsProjective;
use snarkos_models::storage::Storage;
use snarkvm_models::algorithms::CRH;
use snarkvm_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use rand::thread_rng;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Window;

impl PedersenSize for Window {
    const NUM_WINDOWS: usize = 8;
    const WINDOW_SIZE: usize = 128;
}

type TestCRH = PedersenCRH<EdwardsProjective, Window>;

const TEST_CRH_PARAMETERS_PATH: &str = "./pedersen_crh.params";

#[test]
fn crh_parameter_serialization() {
    let rng = &mut thread_rng();

    let crh = TestCRH::setup(rng);

    let crh_bytes = to_bytes![crh].unwrap();
    let recovered_crh: TestCRH = FromBytes::read(&crh_bytes[..]).unwrap();

    assert_eq!(crh, recovered_crh);
}

#[test]
fn crh_parameter_storage() {
    let rng = &mut thread_rng();
    let mut path = std::env::temp_dir();
    path.push(TEST_CRH_PARAMETERS_PATH);

    let crh = TestCRH::setup(rng);
    crh.store(&path).unwrap();

    let recovered_crh = TestCRH::load(&path).unwrap();

    assert_eq!(crh, recovered_crh);

    std::fs::remove_file(&path).unwrap();
}
