use crate::signature::SchnorrSignature;
use snarkos_curves::edwards_sw6::EdwardsAffine as Edwards;
use snarkos_models::{algorithms::SignatureScheme, curves::Group};
use snarkos_utilities::{bytes::ToBytes, rand::UniformRand, to_bytes};

use blake2::Blake2s;
use rand::thread_rng;

fn sign_and_verify<S: SignatureScheme>(message: &[u8]) {
    let rng = &mut thread_rng();
    let parameters = S::setup::<_>(rng).unwrap();
    let (pk, sk) = S::keygen(&parameters, rng).unwrap();
    let sig = S::sign(&parameters, &sk, &message, rng).unwrap();
    assert!(S::verify(&parameters, &pk, &message, &sig).unwrap());
}

fn failed_verification<S: SignatureScheme>(message: &[u8], bad_message: &[u8]) {
    let rng = &mut thread_rng();
    let parameters = S::setup::<_>(rng).unwrap();
    let (pk, sk) = S::keygen(&parameters, rng).unwrap();
    let sig = S::sign(&parameters, &sk, message, rng).unwrap();
    assert!(!S::verify(&parameters, &pk, bad_message, &sig).unwrap());
}

fn randomize_and_verify<S: SignatureScheme>(message: &[u8], randomness: &[u8]) {
    let rng = &mut thread_rng();
    let parameters = S::setup::<_>(rng).unwrap();
    let (pk, sk) = S::keygen(&parameters, rng).unwrap();
    let sig = S::sign(&parameters, &sk, message, rng).unwrap();
    assert!(S::verify(&parameters, &pk, message, &sig).unwrap());
    let randomized_pk = S::randomize_public_key(&parameters, &pk, randomness).unwrap();
    let randomized_sig = S::randomize_signature(&parameters, &sig, randomness).unwrap();
    assert!(S::verify(&parameters, &randomized_pk, &message, &randomized_sig).unwrap());
}

#[test]
fn schnorr_signature_test() {
    let message = "Hi, I am a Schnorr signature!";
    let rng = &mut thread_rng();
    sign_and_verify::<SchnorrSignature<Edwards, Blake2s>>(message.as_bytes());
    failed_verification::<SchnorrSignature<Edwards, Blake2s>>(message.as_bytes(), "Bad message".as_bytes());
    let random_scalar = to_bytes!(<Edwards as Group>::ScalarField::rand(rng)).unwrap();
    randomize_and_verify::<SchnorrSignature<Edwards, Blake2s>>(message.as_bytes(), &random_scalar.as_slice());
}
