use crate::encryption::GroupEncryption;
use snarkos_curves::edwards_bls12::EdwardsProjective;
use snarkos_models::algorithms::EncryptionScheme;
use snarkos_utilities::rand::UniformRand;

use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

type TestEncryptionScheme = GroupEncryption<EdwardsProjective>;

#[test]
fn simple_encryption() {
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);

    let encryption_scheme = TestEncryptionScheme::setup(rng);

    let (private_key, public_key) = encryption_scheme.keygen(rng);

    let message = vec![EdwardsProjective::rand(rng); 32];

    let ciphertext = encryption_scheme.encrypt(&public_key, &message, rng).unwrap();

    let decrypted_message = encryption_scheme.decrypt(&private_key, &ciphertext).unwrap();

    assert_eq!(message, decrypted_message);
}
