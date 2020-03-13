use crate::delegable_payment_dpc::PaymentDPCComponents;

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
use std::{
    io::{Read, Result as IoResult, Write},
    ops::{Add, Mul, Neg},
};

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

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.rbar[..]);
        bytes.extend_from_slice(&self.sbar[..]);

        bytes
    }

    pub fn from_bytes(signature_bytes: Vec<u8>) -> Result<Self, BindingSignatureError> {
        assert_eq!(signature_bytes.len(), 64);

        let rbar = signature_bytes[0..32].to_vec();
        let sbar = signature_bytes[32..64].to_vec();

        let _rbar: <<G as ProjectiveCurve>::Affine as AffineCurve>::BaseField = FromBytes::read(&rbar[..])?;
        let _sbar: <G as Group>::ScalarField = FromBytes::read(&sbar[..])?;

        Ok(Self { rbar, sbar })
    }
}

impl ToBytes for BindingSignature {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.rbar.write(&mut writer)?;
        self.sbar.write(&mut writer)
    }
}

impl FromBytes for BindingSignature {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let rbar: [u8; 32] = FromBytes::read(&mut reader)?;
        let sbar: [u8; 32] = FromBytes::read(&mut reader)?;

        Ok(Self {
            rbar: rbar.to_vec(),
            sbar: sbar.to_vec(),
        })
    }
}

impl Default for BindingSignature {
    #[inline]
    fn default() -> Self {
        Self {
            rbar: [0u8; 32].to_vec(),
            sbar: [0u8; 32].to_vec(),
        }
    }
}

pub fn create_binding_signature<C: PaymentDPCComponents, R: Rng>(
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
    let mut bvk = <G as ProjectiveCurve>::Affine::default();

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

pub fn verify_binding_signature<C: PaymentDPCComponents>(
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
    use crate::delegable_payment_dpc::instantiated::*;

    use snarkos_models::curves::Group;
    use snarkos_utilities::rand::UniformRand;

    fn generate_random_binding_signature<C: PaymentDPCComponents, R: Rng>(
        value_comm_pp: &ValueComm,
        input_amounts: Vec<u64>,
        output_amounts: Vec<u64>,
        sighash: &Vec<u8>,
        rng: &mut R,
    ) -> Result<(Vec<[u8; 32]>, Vec<[u8; 32]>, u64, BindingSignature), BindingSignatureError> {
        let mut value_balance = 0;

        let mut input_value_commitment_randomness = vec![];
        let mut input_value_commitments = vec![];

        let mut output_value_commitment_randomness = vec![];
        let mut output_value_commitments = vec![];

        for input_amount in input_amounts {
            value_balance += input_amount;

            let value_commit_randomness = <G as Group>::ScalarField::rand(rng);
            let value_commitment = value_comm_pp
                .commit(&input_amount.to_le_bytes(), &value_commit_randomness)
                .unwrap();

            let mut value_commitment_randomness_bytes = [0u8; 32];
            let mut value_commitment_bytes = [0u8; 32];

            value_commitment_randomness_bytes.copy_from_slice(&to_bytes![value_commit_randomness].unwrap());
            value_commitment_bytes.copy_from_slice(&to_bytes![value_commitment].unwrap());

            input_value_commitment_randomness.push(value_commitment_randomness_bytes);
            input_value_commitments.push(value_commitment_bytes);
        }

        for output_amount in output_amounts {
            value_balance -= output_amount;

            let value_commit_randomness = <G as Group>::ScalarField::rand(rng);
            let value_commitment = value_comm_pp
                .commit(&output_amount.to_le_bytes(), &value_commit_randomness)
                .unwrap();

            let mut value_commitment_randomness_bytes = [0u8; 32];
            let mut value_commitment_bytes = [0u8; 32];

            value_commitment_randomness_bytes.copy_from_slice(&to_bytes![value_commit_randomness].unwrap());
            value_commitment_bytes.copy_from_slice(&to_bytes![value_commitment].unwrap());

            output_value_commitment_randomness.push(value_commitment_randomness_bytes);
            output_value_commitments.push(value_commitment_bytes);
        }

        let binding_signature = create_binding_signature::<Components, _>(
            value_comm_pp,
            &input_value_commitments,
            &output_value_commitments,
            &input_value_commitment_randomness,
            &output_value_commitment_randomness,
            value_balance,
            sighash,
            rng,
        )
        .unwrap();

        Ok((
            input_value_commitments,
            output_value_commitments,
            value_balance,
            binding_signature,
        ))
    }

    #[test]
    fn test_value_commitment_binding_signature() {
        let rng = &mut rand::thread_rng();

        // Setup parameters

        let comm_and_crh_pp = InstantiatedDPC::generate_comm_and_crh_parameters(rng).unwrap();
        let value_comm_pp = comm_and_crh_pp.value_comm_pp;

        let input_amount: u64 = rng.gen_range(1, 100000000);
        let input_amount_2: u64 = rng.gen_range(1, 100000000);
        let output_amount: u64 = rng.gen_range(0, input_amount);
        let output_amount_2: u64 = rng.gen_range(0, input_amount_2);

        let sighash = [1u8; 64].to_vec();

        let (input_value_commitments, output_value_commitments, value_balance, binding_signature) =
            generate_random_binding_signature::<Components, _>(
                &value_comm_pp,
                vec![input_amount, input_amount_2],
                vec![output_amount, output_amount_2],
                &sighash,
                rng,
            )
            .unwrap();

        // Verify the binding signature

        let verified = verify_binding_signature::<Components>(
            &value_comm_pp,
            &input_value_commitments,
            &output_value_commitments,
            value_balance,
            &sighash,
            &binding_signature,
        )
        .unwrap();

        println!("binding signature verified: {:?}", verified);

        assert!(verified);
    }

    #[test]
    fn test_binding_signature_byte_conversion() {
        let rng = &mut rand::thread_rng();

        // Setup parameters

        let comm_and_crh_pp = InstantiatedDPC::generate_comm_and_crh_parameters(rng).unwrap();
        let value_comm_pp = comm_and_crh_pp.value_comm_pp;

        let input_amount: u64 = rng.gen_range(1, 100000000);
        let input_amount_2: u64 = rng.gen_range(1, 100000000);
        let output_amount: u64 = rng.gen_range(0, input_amount);
        let output_amount_2: u64 = rng.gen_range(0, input_amount_2);

        let sighash = [1u8; 64].to_vec();

        let (_, _, _, binding_signature) = generate_random_binding_signature::<Components, _>(
            &value_comm_pp,
            vec![input_amount, input_amount_2],
            vec![output_amount, output_amount_2],
            &sighash,
            rng,
        )
        .unwrap();

        let binding_signature_bytes = to_bytes![binding_signature].unwrap();
        let reconstructed_binding_signature: BindingSignature = FromBytes::read(&binding_signature_bytes[..]).unwrap();

        assert_eq!(binding_signature, reconstructed_binding_signature);
    }
}
