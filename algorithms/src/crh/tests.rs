use crate::crh::{BoweHopwoodPedersenCRH, PedersenCRH, PedersenCompressedCRH, PedersenSize};
use snarkos_curves::edwards_bls12::EdwardsProjective;
use snarkos_models::algorithms::CRH;
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
    const WINDOW_SIZE: usize = 128;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct BoweHopwoodSize;

impl PedersenSize for BoweHopwoodSize {
    const NUM_WINDOWS: usize = 8;
    const WINDOW_SIZE: usize = 63;
}

fn crh_parameters_serialization<C: CRH>() {
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);

    let crh = C::setup(rng);
    let crh_parameters = crh.parameters();

    let crh_parameters_bytes = to_bytes![crh_parameters].unwrap();
    let recovered_crh_parameters: <C as CRH>::Parameters = FromBytes::read(&crh_parameters_bytes[..]).unwrap();

    assert_eq!(crh_parameters, &recovered_crh_parameters);
}

#[test]
fn pedersen_crh_parameters_serialization() {
    crh_parameters_serialization::<PedersenCRH<EdwardsProjective, Size>>();
}

#[test]
fn pedersen_compressed_crh_parameters_serialization() {
    crh_parameters_serialization::<PedersenCompressedCRH<EdwardsProjective, Size>>();
}

#[test]
fn bowe_hopwood_crh_parameters_serialization() {
    crh_parameters_serialization::<BoweHopwoodPedersenCRH<EdwardsProjective, BoweHopwoodSize>>();
}

#[test]
fn simple_bowe_hopwood_crh() {
    type BoweHopwoodCRH = BoweHopwoodPedersenCRH<EdwardsProjective, BoweHopwoodSize>;

    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);

    let parameters = BoweHopwoodCRH::setup(rng);

    BoweHopwoodCRH::hash(&parameters, &[1, 2, 3]).unwrap();
}
