use crate::encryption::GroupEncryption;
use snarkos_curves::edwards_bls12::EdwardsProjective;
use snarkos_models::{
    algorithms::EncryptionScheme,
    curves::{Group, ProjectiveCurve},
};
use snarkos_utilities::{to_bytes, FromBytes, ToBytes};

use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;

type TestEncryptionScheme = GroupEncryption<EdwardsProjective>;

pub const ITERATIONS: usize = 1000;

fn generate_input<G: Group + ProjectiveCurve, R: Rng>(input_size: usize, rng: &mut R) -> Vec<G> {
    let mut input = vec![];
    for _ in 0..input_size {
        input.push(G::rand(rng))
    }

    input
}

#[test]
fn simple_encryption() {
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);

    let encryption_scheme = TestEncryptionScheme::setup(rng);

    let private_key = encryption_scheme.generate_private_key(rng);
    let public_key = encryption_scheme.generate_public_key(&private_key).unwrap();

    let randomness = encryption_scheme.generate_randomness(&public_key, rng).unwrap();
    let message = generate_input(32, rng);

    let ciphertext = encryption_scheme.encrypt(&public_key, &randomness, &message).unwrap();
    let decrypted_message = encryption_scheme.decrypt(&private_key, &ciphertext).unwrap();

    assert_eq!(message, decrypted_message);
}

#[test]
fn encryption_public_key_serialization() {
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);

    let encryption_scheme = TestEncryptionScheme::setup(rng);

    for _ in 0..ITERATIONS {
        let private_key = encryption_scheme.generate_private_key(rng);
        let public_key = encryption_scheme.generate_public_key(&private_key).unwrap();

        let public_key_bytes = to_bytes![public_key].unwrap();
        let recovered_public_key =
            <TestEncryptionScheme as EncryptionScheme>::PublicKey::read(&public_key_bytes[..]).unwrap();

        assert_eq!(public_key, recovered_public_key);
    }
}
