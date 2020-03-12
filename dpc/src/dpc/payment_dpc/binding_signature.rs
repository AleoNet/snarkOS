use crate::payment_dpc::PlainDPCComponents;

use snarkos_curves::edwards_bls12::EdwardsProjective as EdwardsBls12;
use snarkos_errors::dpc::BindingSignatureError;
use snarkos_models::{
    algorithms::CommitmentScheme,
    curves::{
        pairing_engine::{AffineCurve, ProjectiveCurve},
        Field,
        Group,
    },
};
use snarkos_utilities::{
    bititerator::BitIterator,
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use blake2::{
    digest::{Input, VariableOutput},
    VarBlake2b as b2s,
};
use rand::Rng;
use std::ops::{Add, Mul, Neg};

type G = EdwardsBls12;

pub fn hash_into_field<G: Group + ProjectiveCurve>(a: &[u8], b: &[u8]) -> <G as Group>::ScalarField {
    let mut hasher = b2s::new(64).unwrap();
    hasher.input(a);
    hasher.input(b);
    let hash: Vec<u8> = hasher.vec_result();

    let hash_u64_repr: Vec<u64> = hash
        .chunks(8)
        .map(|chunk| {
            let mut fixed_size = [0u8; 8];
            fixed_size.copy_from_slice(chunk);
            u64::from_le_bytes(fixed_size)
        })
        .collect();

    // Scaling by random cofactor for the scalar field
    let mut res = <G as Group>::ScalarField::one();
    for bit in BitIterator::new(hash_u64_repr) {
        res.double_in_place();
        if bit {
            res = res.add(&res)
        }
    }

    res
}

pub fn recover_affine_from_x_coord(x_bytes: &[u8]) -> Result<<G as ProjectiveCurve>::Affine, BindingSignatureError> {
    let x: <<G as ProjectiveCurve>::Affine as AffineCurve>::BaseField = FromBytes::read(x_bytes)?;

    if let Some(affine) = <G as ProjectiveCurve>::Affine::get_point_from_x(x, false) {
        if affine.is_in_correct_subgroup_assuming_on_curve() {
            return Ok(affine);
        }
    }

    if let Some(affine) = <G as ProjectiveCurve>::Affine::get_point_from_x(x, true) {
        if affine.is_in_correct_subgroup_assuming_on_curve() {
            return Ok(affine);
        }
    }

    Err(BindingSignatureError::NotInCorrectSubgroupOnCurve(to_bytes![x]?))
}

// Binding signature scheme derived from Zcash's redDSA
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BindingSignature {
    pub rbar: Vec<u8>,
    pub sbar: Vec<u8>,
}

impl BindingSignature {
    pub fn new(rbar: Vec<u8>, sbar: Vec<u8>) -> Result<Self, BindingSignatureError> {
        assert_eq!(rbar.len(), 32);
        assert_eq!(sbar.len(), 32);

        Ok(Self { rbar, sbar })
    }
}

pub fn create_binding_signature<C: PlainDPCComponents, R: Rng>(
    parameters: &C::ValueComm,
    input_value_commitments: &Vec<[u8; 32]>,
    output_value_commitments: &Vec<[u8; 32]>,
    input_value_commitment_randomness: &Vec<[u8; 32]>,
    output_value_commitment_randomness: &Vec<[u8; 32]>,
    value_balance: u64,
    input: &Vec<u8>,
    rng: &mut R,
) -> Result<BindingSignature, BindingSignatureError> {
    // Calculate Value balance commitment
    let zero_randomness = <C::ValueComm as CommitmentScheme>::Randomness::default();
    let value_balance_commitment = to_bytes![parameters.commit(&value_balance.to_le_bytes(), &zero_randomness)?]?;

    // Calculate the bsk and bvk
    let mut bsk = <G as Group>::ScalarField::default();
    let mut bvk = <EdwardsBls12 as ProjectiveCurve>::Affine::default();

    for input_vc_randomness in input_value_commitment_randomness {
        let randomness: <G as Group>::ScalarField = FromBytes::read(&input_vc_randomness[..])?;
        bsk = bsk.add(&randomness);
    }

    for output_vc_randomness in output_value_commitment_randomness {
        let randomness: <G as Group>::ScalarField = FromBytes::read(&output_vc_randomness[..])?;
        bsk = bsk.add(&randomness.neg());
    }

    for vc_input in input_value_commitments {
        let recovered_input_value_commitment = recover_affine_from_x_coord(&vc_input[..])?;
        bvk = bvk.add(&recovered_input_value_commitment);
    }

    for vc_output in output_value_commitments {
        let recovered_output_value_commitment = recover_affine_from_x_coord(&vc_output[..])?;
        bvk = bvk.add(&recovered_output_value_commitment.neg());
    }

    let recovered_value_balance_commitment = recover_affine_from_x_coord(&value_balance_commitment)?;
    bvk = bvk.add(&recovered_value_balance_commitment.neg());

    // Make sure bvk can be derived from bsk
    let zero: u64 = 0;
    let comm_bsk: <C::ValueComm as CommitmentScheme>::Randomness = FromBytes::read(&to_bytes![bsk]?[..])?;
    let expected_bvk_x = to_bytes![parameters.commit(&zero.to_le_bytes(), &comm_bsk)?]?;
    let expected_bvk = recover_affine_from_x_coord(&expected_bvk_x)?;
    assert_eq!(bvk, expected_bvk);

    // Generate randomness
    let mut sig_rand = [0u8; 80];
    rng.fill(&mut sig_rand[..]);

    // Generate signature using message

    let r_edwards: <G as Group>::ScalarField = hash_into_field::<G>(&sig_rand[..], input);
    let r: <C::ValueComm as CommitmentScheme>::Randomness = FromBytes::read(&to_bytes![r_edwards]?[..])?;
    let r_g = parameters.commit(&zero.to_le_bytes(), &r)?;

    let mut rbar = [0u8; 32];
    r_g.write(&mut rbar[..])?;

    let mut s: <G as Group>::ScalarField = hash_into_field::<G>(&rbar[..], input);
    s = s.mul(&bsk);
    s = s.add(&r_edwards);

    let mut sbar = [0u8; 32];
    sbar.copy_from_slice(&to_bytes![s]?[..]);

    BindingSignature::new(rbar.to_vec(), sbar.to_vec())
}

pub fn verify_binding_signature<C: PlainDPCComponents>(
    parameters: &C::ValueComm,
    input_value_commitments: &Vec<[u8; 32]>,
    output_value_commitments: &Vec<[u8; 32]>,
    value_balance: u64,
    input: &Vec<u8>,
    signature: &BindingSignature,
) -> Result<bool, BindingSignatureError> {
    // Calculate Value balance commitment
    let zero_randomness = <C::ValueComm as CommitmentScheme>::Randomness::default();
    let value_balance_commitment = to_bytes![parameters.commit(&value_balance.to_le_bytes(), &zero_randomness)?]?;

    // Craft verifying key
    let mut bvk = <G as ProjectiveCurve>::Affine::default();

    for vc_input in input_value_commitments {
        let recovered_input_value_commitment = recover_affine_from_x_coord(&vc_input[..])?;
        bvk = bvk.add(&recovered_input_value_commitment);
    }

    for vc_output in output_value_commitments {
        let recovered_output_value_commitment = recover_affine_from_x_coord(&vc_output[..])?;
        bvk = bvk.add(&recovered_output_value_commitment.neg());
    }

    let recovered_value_balance_commitment = recover_affine_from_x_coord(&value_balance_commitment)?;
    bvk = bvk.add(&recovered_value_balance_commitment.neg());

    //Verify the signature
    let c: <G as Group>::ScalarField = hash_into_field::<G>(&signature.rbar[..], input);
    let affine_r = recover_affine_from_x_coord(&signature.rbar)?;

    let zero: u64 = 0;
    let s: <C::ValueComm as CommitmentScheme>::Randomness = FromBytes::read(&signature.sbar[..])?;
    let recommit = to_bytes![parameters.commit(&zero.to_le_bytes(), &s)?]?;
    let recovered_recommit = recover_affine_from_x_coord(&recommit).unwrap();

    let check_verification = bvk.mul(&c).add(&affine_r).add(&recovered_recommit.neg());

    Ok(<<G as ProjectiveCurve>::Affine as AffineCurve>::is_zero(
        &check_verification,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payment_dpc::instantiated::*;

    use snarkos_models::curves::Group;
    use snarkos_utilities::rand::UniformRand;

    #[test]
    fn test_value_commitment_binding_signature() {
        let rng = &mut rand::thread_rng();

        // Setup parameters

        let comm_and_crh_pp = InstantiatedDPC::generate_comm_and_crh_parameters(rng).unwrap();
        let value_comm_pp = comm_and_crh_pp.value_comm_pp;

        let input_amount: u64 = rng.gen_range(1, 100000);
        let output_amount: u64 = rng.gen_range(0, input_amount);

        let value_balance = input_amount - output_amount;

        // Input value commitment

        let input_value_commitment_randomness = <G as Group>::ScalarField::rand(rng);
        let input_value_commitment = value_comm_pp
            .commit(&input_amount.to_le_bytes(), &input_value_commitment_randomness)
            .unwrap();

        // Output value commitment

        let output_value_commitment_randomness = <G as Group>::ScalarField::rand(rng);
        let output_value_commitment = value_comm_pp
            .commit(&output_amount.to_le_bytes(), &output_value_commitment_randomness)
            .unwrap();

        // Create the binding signature

        let mut input_value_commitment_randomness_bytes = [0u8; 32];
        let mut output_value_commitment_randomness_bytes = [0u8; 32];
        let mut input_value_commitment_bytes = [0u8; 32];
        let mut output_value_commitment_bytes = [0u8; 32];

        input_value_commitment_randomness_bytes.copy_from_slice(&to_bytes![input_value_commitment_randomness].unwrap());
        output_value_commitment_randomness_bytes
            .copy_from_slice(&to_bytes![output_value_commitment_randomness].unwrap());

        input_value_commitment_bytes.copy_from_slice(&to_bytes![input_value_commitment].unwrap());
        output_value_commitment_bytes.copy_from_slice(&to_bytes![output_value_commitment].unwrap());

        let sighash = [1u8; 64].to_vec();

        let binding_signature = create_binding_signature::<Components, _>(
            &value_comm_pp,
            &vec![input_value_commitment_bytes],
            &vec![output_value_commitment_bytes],
            &vec![input_value_commitment_randomness_bytes],
            &vec![output_value_commitment_randomness_bytes],
            value_balance,
            &sighash,
            rng,
        )
        .unwrap();

        // Verify the binding signature

        let verified = verify_binding_signature::<Components>(
            &value_comm_pp,
            &vec![input_value_commitment_bytes],
            &vec![output_value_commitment_bytes],
            value_balance,
            &sighash,
            &binding_signature,
        )
        .unwrap();

        println!("binding signature verified: {:?}", verified);

        assert!(verified);
    }
}
