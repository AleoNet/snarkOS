use crate::dpc::base_dpc::BaseDPCComponents;
use snarkos_models::algorithms::{CommitmentScheme, SignatureScheme, CRH, SNARK};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use std::{
    fs::File,
    io::{Result as IoResult, Write},
    path::PathBuf,
};

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct CircuitParameters<C: BaseDPCComponents> {
    pub account_commitment: C::AccountCommitment,
    pub account_signature: C::AccountSignature,
    pub record_commitment: C::RecordCommitment,
    pub predicate_verification_key_commitment: C::PredicateVerificationKeyCommitment,
    pub predicate_verification_key_hash: C::PredicateVerificationKeyHash,
    pub local_data_commitment: C::LocalDataCommitment,
    pub value_commitment: C::ValueCommitment,
    pub serial_number_nonce: C::SerialNumberNonceCRH,
}

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct PredicateSNARKParameters<C: BaseDPCComponents> {
    pub proving_key: <C::PredicateSNARK as SNARK>::ProvingParameters,
    pub verification_key: <C::PredicateSNARK as SNARK>::VerificationParameters,
    pub proof: <C::PredicateSNARK as SNARK>::Proof,
}

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct PublicParameters<C: BaseDPCComponents> {
    pub circuit_parameters: CircuitParameters<C>,
    pub predicate_snark_parameters: PredicateSNARKParameters<C>,
    pub outer_snark_parameters: (
        Option<<C::OuterSNARK as SNARK>::ProvingParameters>,
        <C::OuterSNARK as SNARK>::PreparedVerificationParameters,
    ),
    pub inner_snark_parameters: (
        Option<<C::InnerSNARK as SNARK>::ProvingParameters>,
        <C::InnerSNARK as SNARK>::PreparedVerificationParameters,
    ),
}

impl<C: BaseDPCComponents> PublicParameters<C> {
    pub fn account_commitment_parameters(&self) -> &C::AccountCommitment {
        &self.circuit_parameters.account_commitment
    }

    pub fn account_signature_parameters(&self) -> &C::AccountSignature {
        &self.circuit_parameters.account_signature
    }

    pub fn inner_snark_parameters(
        &self,
    ) -> &(
        Option<<C::InnerSNARK as SNARK>::ProvingParameters>,
        <C::InnerSNARK as SNARK>::PreparedVerificationParameters,
    ) {
        &self.inner_snark_parameters
    }

    pub fn local_data_commitment_parameters(&self) -> &C::LocalDataCommitment {
        &self.circuit_parameters.local_data_commitment
    }

    pub fn outer_snark_parameters(
        &self,
    ) -> &(
        Option<<C::OuterSNARK as SNARK>::ProvingParameters>,
        <C::OuterSNARK as SNARK>::PreparedVerificationParameters,
    ) {
        &self.outer_snark_parameters
    }

    pub fn predicate_snark_parameters(&self) -> &PredicateSNARKParameters<C> {
        &self.predicate_snark_parameters
    }

    pub fn predicate_verification_key_commitment_parameters(&self) -> &C::PredicateVerificationKeyCommitment {
        &self.circuit_parameters.predicate_verification_key_commitment
    }

    pub fn predicate_verification_key_hash_parameters(&self) -> &C::PredicateVerificationKeyHash {
        &self.circuit_parameters.predicate_verification_key_hash
    }

    pub fn record_commitment_parameters(&self) -> &C::RecordCommitment {
        &self.circuit_parameters.record_commitment
    }

    pub fn value_commitment_parameters(&self) -> &C::ValueCommitment {
        &self.circuit_parameters.value_commitment
    }

    pub fn serial_number_nonce_parameters(&self) -> &C::SerialNumberNonceCRH {
        &self.circuit_parameters.serial_number_nonce
    }

    pub fn store(&self, parameter_dir: &PathBuf) -> IoResult<()> {
        let circuit_dir = parameter_dir.join("circuit/");

        fn store_bytes(parameters_bytes: Vec<u8>, path: &PathBuf) -> IoResult<()> {
            let mut file = File::create(path)?;
            file.write_all(&parameters_bytes)?;
            drop(file);
            Ok(())
        }

        // Circuit Parameters

        let account_commitment_parameters_path = &circuit_dir.join("account_commitment.params");
        let account_signature_parameters_path = &circuit_dir.join("account_signature.params");
        let record_commitment_parameters_path = &circuit_dir.join("record_commitment.params");
        let predicate_vk_commitment_parameters_path = &circuit_dir.join("predicate_vk_commitment.params");
        let predicate_vk_crh_parameters_path = &circuit_dir.join("predicate_vk_crh.params");
        let local_data_commitment_parameters_path = &circuit_dir.join("local_data_commitment.params");
        let value_commitment_parameters_path = &circuit_dir.join("value_commitment.params");
        let serial_number_nonce_crh_parameters_path = &circuit_dir.join("serial_number_nonce_crh.params");

        let circuit_parameters = &self.circuit_parameters;

        let account_commitment_parameters_bytes = to_bytes![circuit_parameters.account_commitment.parameters()]?;
        let account_signature_parameters_bytes = to_bytes![circuit_parameters.account_signature.parameters()]?;
        let record_commitment_parameters_bytes = to_bytes![circuit_parameters.record_commitment.parameters()]?;
        let predicate_vk_commitment_parameters_bytes =
            to_bytes![circuit_parameters.predicate_verification_key_commitment.parameters()]?;
        let predicate_vk_crh_parameters_bytes =
            to_bytes![circuit_parameters.predicate_verification_key_hash.parameters()]?;
        let local_data_commitment_parameters_bytes = to_bytes![circuit_parameters.local_data_commitment.parameters()]?;
        let value_commitment_parameters_bytes = to_bytes![circuit_parameters.value_commitment.parameters()]?;
        let serial_number_nonce_crh_parameters_bytes = to_bytes![circuit_parameters.serial_number_nonce.parameters()]?;

        store_bytes(account_commitment_parameters_bytes, account_commitment_parameters_path)?;
        store_bytes(account_signature_parameters_bytes, account_signature_parameters_path)?;
        store_bytes(record_commitment_parameters_bytes, record_commitment_parameters_path)?;
        store_bytes(
            predicate_vk_commitment_parameters_bytes,
            predicate_vk_commitment_parameters_path,
        )?;
        store_bytes(predicate_vk_crh_parameters_bytes, predicate_vk_crh_parameters_path)?;
        store_bytes(
            local_data_commitment_parameters_bytes,
            local_data_commitment_parameters_path,
        )?;
        store_bytes(value_commitment_parameters_bytes, value_commitment_parameters_path)?;
        store_bytes(
            serial_number_nonce_crh_parameters_bytes,
            serial_number_nonce_crh_parameters_path,
        )?;

        // Predicate SNARK Parameters

        let predicate_snark_pk_path = &parameter_dir.join("predicate_snark.params");
        let predicate_snark_vk_path = &parameter_dir.join("predicate_snark_vk.params");
        let predicate_snark_proof_path = &parameter_dir.join("predicate_snark.proof");

        let predicate_snark_pk_bytes = to_bytes![self.predicate_snark_parameters.proving_key]?;
        let predicate_snark_vk_bytes = to_bytes![self.predicate_snark_parameters.verification_key]?;
        let predicate_snark_proof_bytes = to_bytes![self.predicate_snark_parameters.proof]?;

        store_bytes(predicate_snark_pk_bytes, predicate_snark_pk_path)?;
        store_bytes(predicate_snark_vk_bytes, predicate_snark_vk_path)?;
        store_bytes(predicate_snark_proof_bytes, predicate_snark_proof_path)?;

        // Outer SNARK parameters

        let outer_snark_pk_path = &parameter_dir.join("outer_snark.params");
        let outer_snark_vk_path = &parameter_dir.join("outer_snark_vk.params");

        if let Some(parameters) = &self.outer_snark_parameters.0 {
            let outer_snark_pk_bytes = to_bytes![parameters]?;
            store_bytes(outer_snark_pk_bytes, outer_snark_pk_path)?;
        };

        let outer_snark_vk: <C::OuterSNARK as SNARK>::VerificationParameters =
            self.outer_snark_parameters.1.clone().into();
        let outer_snark_vk_bytes = to_bytes![outer_snark_vk]?;

        store_bytes(outer_snark_vk_bytes, outer_snark_vk_path)?;

        // Inner SNARK parameters

        let inner_snark_pk_path = &parameter_dir.join("inner_snark.params");
        let inner_snark_vk_path = &parameter_dir.join("inner_snark_vk.params");

        if let Some(parameters) = &self.inner_snark_parameters.0 {
            let inner_snark_pk_parameters_bytes = to_bytes![parameters]?;
            store_bytes(inner_snark_pk_parameters_bytes, inner_snark_pk_path)?;
        };

        let inner_snark_vk: <C::InnerSNARK as SNARK>::VerificationParameters =
            self.inner_snark_parameters.1.clone().into();
        let inner_snark_vk_bytes = to_bytes![inner_snark_vk]?;

        store_bytes(inner_snark_vk_bytes, inner_snark_vk_path)?;

        Ok(())
    }

    pub fn load(dir_path: &PathBuf, verify_only: bool) -> IoResult<Self> {
        let circuit_dir = dir_path.join("circuit/");

        fn load_commitment<C: CommitmentScheme>(path: &PathBuf) -> IoResult<C> {
            let mut file = File::open(path)?;
            let parameters: <C as CommitmentScheme>::Parameters = FromBytes::read(&mut file)?;
            Ok(C::from(parameters))
        }

        fn load_crh<C: CRH>(path: &PathBuf) -> IoResult<C> {
            let mut file = File::open(path)?;
            let parameters: <C as CRH>::Parameters = FromBytes::read(&mut file)?;
            Ok(C::from(parameters))
        }

        fn load_signature_scheme<S: SignatureScheme>(path: &PathBuf) -> IoResult<S> {
            let mut file = File::open(path)?;
            let parameters: <S as SignatureScheme>::Parameters = FromBytes::read(&mut file)?;
            Ok(S::from(parameters))
        }

        fn load_snark_pk<S: SNARK>(path: &PathBuf) -> IoResult<S::ProvingParameters> {
            let mut file = File::open(path)?;
            let proving_parameters: <S as SNARK>::ProvingParameters = FromBytes::read(&mut file)?;
            Ok(proving_parameters)
        }

        fn load_snark_vk<S: SNARK>(path: &PathBuf) -> IoResult<S::VerificationParameters> {
            let mut file = File::open(path)?;
            let verification_parameters: <S as SNARK>::VerificationParameters = FromBytes::read(&mut file)?;
            Ok(verification_parameters)
        }

        fn load_snark_proof<S: SNARK>(path: &PathBuf) -> IoResult<S::Proof> {
            let mut file = File::open(path)?;
            let proof: <S as SNARK>::Proof = FromBytes::read(&mut file)?;
            Ok(proof)
        }

        // Circuit Parameters
        let circuit_parameters: CircuitParameters<C> = {
            let account_commitment_parameters_path = &circuit_dir.join("account_commitment.params");
            let account_signature_parameters_path = &circuit_dir.join("account_signature.params");
            let record_commitment_parameters_path = &circuit_dir.join("record_commitment.params");
            let predicate_vk_commitment_parameters_path = &circuit_dir.join("predicate_vk_commitment.params");
            let predicate_vk_crh_parameters_path = &circuit_dir.join("predicate_vk_crh.params");
            let local_data_commitment_parameters_path = &circuit_dir.join("local_data_commitment.params");
            let value_commitment_parameters_path = &circuit_dir.join("value_commitment.params");
            let serial_number_nonce_crh_parameters_path = &circuit_dir.join("serial_number_nonce_crh.params");

            let account_commitment = load_commitment::<C::AccountCommitment>(account_commitment_parameters_path)?;
            let account_signature = load_signature_scheme::<C::AccountSignature>(account_signature_parameters_path)?;
            let record_commitment = load_commitment::<C::RecordCommitment>(record_commitment_parameters_path)?;
            let predicate_verification_key_commitment =
                load_commitment::<C::PredicateVerificationKeyCommitment>(predicate_vk_commitment_parameters_path)?;
            let predicate_verification_key_hash =
                load_crh::<C::PredicateVerificationKeyHash>(predicate_vk_crh_parameters_path)?;
            let local_data_commitment =
                load_commitment::<C::LocalDataCommitment>(local_data_commitment_parameters_path)?;
            let value_commitment = load_commitment::<C::ValueCommitment>(value_commitment_parameters_path)?;
            let serial_number_nonce = load_crh::<C::SerialNumberNonceCRH>(serial_number_nonce_crh_parameters_path)?;

            CircuitParameters::<C> {
                account_commitment,
                account_signature,
                record_commitment,
                predicate_verification_key_commitment,
                predicate_verification_key_hash,
                local_data_commitment,
                value_commitment,
                serial_number_nonce,
            }
        };

        // SNARK Parameters

        let predicate_snark_parameters: PredicateSNARKParameters<C> = {
            let predicate_snark_pk_path = &dir_path.join("predicate_snark.params");
            let predicate_snark_vk_path = &dir_path.join("predicate_snark_vk.params");
            let predicate_snark_proof_path = &dir_path.join("predicate_snark.proof");

            let proving_key = load_snark_pk::<C::PredicateSNARK>(predicate_snark_pk_path)?;
            let verification_key = load_snark_vk::<C::PredicateSNARK>(predicate_snark_vk_path)?;
            let proof = load_snark_proof::<C::PredicateSNARK>(predicate_snark_proof_path)?;

            PredicateSNARKParameters::<C> {
                proving_key,
                verification_key,
                proof,
            }
        };

        let outer_snark_parameters: (
            Option<<C::OuterSNARK as SNARK>::ProvingParameters>,
            <C::OuterSNARK as SNARK>::PreparedVerificationParameters,
        ) = {
            let outer_snark_pk_path = &dir_path.join("outer_snark.params");
            let outer_snark_vk_path = &dir_path.join("outer_snark_vk.params");

            let outer_snark_pk = match verify_only {
                true => None,
                false => Some(load_snark_pk::<C::OuterSNARK>(outer_snark_pk_path)?),
            };

            let outer_snark_vk = load_snark_vk::<C::OuterSNARK>(outer_snark_vk_path)?;
            let outer_snark_prepared_vk = outer_snark_vk.into();

            (outer_snark_pk, outer_snark_prepared_vk)
        };

        let inner_snark_parameters: (
            Option<<C::InnerSNARK as SNARK>::ProvingParameters>,
            <C::InnerSNARK as SNARK>::PreparedVerificationParameters,
        ) = {
            let inner_snark_pk_path = &dir_path.join("inner_snark.params");
            let inner_snark_vk_path = &dir_path.join("inner_snark_vk.params");

            let inner_snark_pk = match verify_only {
                true => None,
                false => Some(load_snark_pk::<C::InnerSNARK>(inner_snark_pk_path)?),
            };

            let inner_snark_vk = load_snark_vk::<C::InnerSNARK>(inner_snark_vk_path)?;
            let inner_snark_prepared_vk = inner_snark_vk.into();

            (inner_snark_pk, inner_snark_prepared_vk)
        };

        Ok(Self {
            circuit_parameters,
            predicate_snark_parameters,
            outer_snark_parameters,
            inner_snark_parameters,
        })
    }

    pub fn load_vk_direct() -> IoResult<Self> {
        // Circuit Parameters
        let circuit_parameters: CircuitParameters<C> = {
            let account_commitment_parameters = include_bytes!["../../parameters/circuit/account_commitment.params"];
            let record_commitment_parameters = include_bytes!["../../parameters/circuit/record_commitment.params"];
            let predicate_vk_commitment_parameters = vec![];
            let predicate_vk_crh_parameters = include_bytes!["../../parameters/circuit/predicate_vk_crh.params"];
            let local_data_commitment_parameters =
                include_bytes!["../../parameters/circuit/local_data_commitment.params"];
            let value_commitment_parameters = include_bytes!["../../parameters/circuit/value_commitment.params"];
            let serial_number_nonce_crh_parameters =
                include_bytes!["../../parameters/circuit/serial_number_nonce_crh.params"];
            let account_signature_parameters = include_bytes!["../../parameters/circuit/account_signature.params"];

            fn load_commitment<C: CommitmentScheme>(parameters_bytes: &[u8]) -> IoResult<C> {
                let parameters: <C as CommitmentScheme>::Parameters = FromBytes::read(&parameters_bytes[..])?;
                Ok(C::from(parameters))
            }

            fn load_crh<C: CRH>(parameters_bytes: &[u8]) -> IoResult<C> {
                let parameters: <C as CRH>::Parameters = FromBytes::read(&parameters_bytes[..])?;
                Ok(C::from(parameters))
            }

            fn load_signature_scheme<S: SignatureScheme>(parameters_bytes: &[u8]) -> IoResult<S> {
                let parameters: <S as SignatureScheme>::Parameters = FromBytes::read(&parameters_bytes[..])?;
                Ok(S::from(parameters))
            }

            let account_commitment = load_commitment::<C::AccountCommitment>(account_commitment_parameters)?;
            let account_signature = load_signature_scheme::<C::AccountSignature>(account_signature_parameters)?;
            let record_commitment = load_commitment::<C::RecordCommitment>(record_commitment_parameters)?;
            let predicate_verification_key_commitment =
                load_commitment::<C::PredicateVerificationKeyCommitment>(&predicate_vk_commitment_parameters)?;
            let predicate_verification_key_hash =
                load_crh::<C::PredicateVerificationKeyHash>(predicate_vk_crh_parameters)?;
            let local_data_commitment = load_commitment::<C::LocalDataCommitment>(local_data_commitment_parameters)?;
            let value_commitment = load_commitment::<C::ValueCommitment>(value_commitment_parameters)?;
            let serial_number_nonce = load_crh::<C::SerialNumberNonceCRH>(serial_number_nonce_crh_parameters)?;

            CircuitParameters::<C> {
                account_commitment,
                account_signature,
                record_commitment,
                predicate_verification_key_commitment,
                predicate_verification_key_hash,
                local_data_commitment,
                value_commitment,
                serial_number_nonce,
            }
        };

        // SNARK Parameters

        let predicate_snark_parameters: PredicateSNARKParameters<C> = {
            let predicate_snark_pk_bytes = include_bytes!["../../parameters/predicate_snark.params"];
            let predicate_snark_vk_bytes = include_bytes!["../../parameters/predicate_snark_vk.params"];
            let predicate_snark_proof_bytes = include_bytes!["../../parameters/predicate_snark.proof"];

            let proving_key: <C::PredicateSNARK as SNARK>::ProvingParameters =
                FromBytes::read(&predicate_snark_pk_bytes[..])?;
            let verification_key: <C::PredicateSNARK as SNARK>::VerificationParameters =
                FromBytes::read(&predicate_snark_vk_bytes[..])?;
            let proof: <C::PredicateSNARK as SNARK>::Proof = FromBytes::read(&predicate_snark_proof_bytes[..])?;

            PredicateSNARKParameters::<C> {
                proving_key,
                verification_key,
                proof,
            }
        };

        let outer_snark_parameters: (
            Option<<C::OuterSNARK as SNARK>::ProvingParameters>,
            <C::OuterSNARK as SNARK>::PreparedVerificationParameters,
        ) = {
            let outer_snark_pk = None;

            let outer_snark_vk_bytes = include_bytes!["../../parameters/outer_snark_vk.params"];
            let outer_snark_vk: <C::OuterSNARK as SNARK>::VerificationParameters =
                FromBytes::read(&outer_snark_vk_bytes[..])?;
            let outer_snark_prepared_vk = outer_snark_vk.into();

            (outer_snark_pk, outer_snark_prepared_vk)
        };

        let inner_snark_parameters: (
            Option<<C::InnerSNARK as SNARK>::ProvingParameters>,
            <C::InnerSNARK as SNARK>::PreparedVerificationParameters,
        ) = {
            let inner_snark_pk = None;

            let inner_snark_vk_bytes = include_bytes!["../../parameters/inner_snark_vk.params"];
            let inner_snark_vk: <C::InnerSNARK as SNARK>::VerificationParameters =
                FromBytes::read(&inner_snark_vk_bytes[..])?;
            let inner_snark_prepared_vk = inner_snark_vk.into();

            (inner_snark_pk, inner_snark_prepared_vk)
        };

        Ok(Self {
            circuit_parameters,
            predicate_snark_parameters,
            outer_snark_parameters,
            inner_snark_parameters,
        })
    }
}
