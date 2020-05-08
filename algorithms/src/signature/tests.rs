use crate::signature::SchnorrSignature;
use snarkos_curves::edwards_sw6::EdwardsAffine as Edwards;
use snarkos_models::{algorithms::SignatureScheme, curves::Group, storage::Storage};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    rand::UniformRand,
    to_bytes,
};

use blake2::Blake2s;
use rand::thread_rng;

type TestSignature = SchnorrSignature<Edwards, Blake2s>;

const TEST_SIGNATURE_PARAMETERS_PATH: &str = "./schnorr_signature.params";

fn sign_and_verify<S: SignatureScheme>(message: &[u8]) {
    let rng = &mut thread_rng();
    let schnorr_signature = S::setup::<_>(rng).unwrap();
    let private_key = schnorr_signature.generate_private_key(rng).unwrap();
    let public_key = schnorr_signature.generate_public_key(&private_key).unwrap();
    let signature = schnorr_signature.sign(&private_key, message, rng).unwrap();
    assert!(schnorr_signature.verify(&public_key, &message, &signature).unwrap());
}

fn failed_verification<S: SignatureScheme>(message: &[u8], bad_message: &[u8]) {
    let rng = &mut thread_rng();
    let schnorr_signature = S::setup::<_>(rng).unwrap();
    let private_key = schnorr_signature.generate_private_key(rng).unwrap();
    let public_key = schnorr_signature.generate_public_key(&private_key).unwrap();
    let signature = schnorr_signature.sign(&private_key, message, rng).unwrap();
    assert!(!schnorr_signature.verify(&public_key, bad_message, &signature).unwrap());
}

fn randomize_and_verify<S: SignatureScheme>(message: &[u8], randomness: &[u8]) {
    let rng = &mut thread_rng();
    let schnorr_signature = S::setup::<_>(rng).unwrap();
    let private_key = schnorr_signature.generate_private_key(rng).unwrap();
    let public_key = schnorr_signature.generate_public_key(&private_key).unwrap();
    let signature = schnorr_signature.sign(&private_key, message, rng).unwrap();
    assert!(schnorr_signature.verify(&public_key, message, &signature).unwrap());

    let randomized_public_key = schnorr_signature.randomize_public_key(&public_key, randomness).unwrap();
    let randomized_signature = schnorr_signature.randomize_signature(&signature, randomness).unwrap();
    assert!(
        schnorr_signature
            .verify(&randomized_public_key, &message, &randomized_signature)
            .unwrap()
    );
}

#[test]
fn schnorr_signature_test() {
    let message = "Hi, I am a Schnorr signature!";
    let rng = &mut thread_rng();
    sign_and_verify::<TestSignature>(message.as_bytes());
    failed_verification::<TestSignature>(message.as_bytes(), "Bad message".as_bytes());
    let random_scalar = to_bytes!(<Edwards as Group>::ScalarField::rand(rng)).unwrap();
    randomize_and_verify::<TestSignature>(message.as_bytes(), &random_scalar.as_slice());
}

#[test]
fn schnorr_signature_parameter_serialization() {
    let rng = &mut thread_rng();

    let schnorr_signature = TestSignature::setup(rng).unwrap();

    let schnorr_signature_bytes = to_bytes![schnorr_signature].unwrap();

    let recovered_schnorr_signature: TestSignature = FromBytes::read(&schnorr_signature_bytes[..]).unwrap();

    assert_eq!(schnorr_signature, recovered_schnorr_signature);
}

#[test]
fn schnorr_signature_parameter_storage() {
    let rng = &mut thread_rng();
    let mut path = std::env::temp_dir();
    path.push(TEST_SIGNATURE_PARAMETERS_PATH);

    let schnorr_signature = TestSignature::setup(rng).unwrap();

    schnorr_signature.store(&path).unwrap();

    let recovered_schnorr_signature = TestSignature::load(&path).unwrap();

    assert_eq!(schnorr_signature, recovered_schnorr_signature);

    std::fs::remove_file(&path).unwrap();
}
