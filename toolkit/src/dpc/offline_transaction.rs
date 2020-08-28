use crate::{
    account::{Address, PrivateKey},
    dpc::{EmptyLedger, Record},
    errors::DPCError,
};
use snarkos_dpc::base_dpc::{
    instantiated::{CommitmentMerkleParameters, Components, InstantiatedDPC, Tx},
    parameters::{NoopProgramSNARKParameters, SystemParameters},
    record_payload::RecordPayload,
    ExecuteContext,
};
use snarkos_models::{
    algorithms::CRH,
    dpc::{DPCComponents, DPCScheme, Record as RecordScheme},
};
use snarkos_objects::account::*;
use snarkos_utilities::{to_bytes, FromBytes, ToBytes};

use rand::Rng;
use std::{fmt, str::FromStr};

pub type MerkleTreeLedger = EmptyLedger<Tx, CommitmentMerkleParameters>;

pub struct OfflineTransaction {
    pub(crate) execute_context: ExecuteContext<Components>,
}

impl OfflineTransaction {
    /// Returns an offline transaction execution context
    pub fn offline_transaction_execution<R: Rng>(
        spenders: Vec<PrivateKey>,
        records_to_spend: Vec<Record>,
        recipients: Vec<Address>,
        recipient_amounts: Vec<u64>,
        network_id: u8,
        rng: &mut R,
    ) -> Result<Self, DPCError> {
        let parameters = SystemParameters::<Components>::load().unwrap();

        let noop_program_snark_parameters = NoopProgramSNARKParameters::<Components>::load().unwrap();
        assert!(spenders.len() > 0);
        assert_eq!(spenders.len(), records_to_spend.len());

        assert!(recipients.len() > 0);
        assert_eq!(recipients.len(), recipient_amounts.len());

        let noop_program_id = to_bytes![
            parameters
                .program_verification_key_crh
                .hash(&to_bytes![noop_program_snark_parameters.verification_key]?)?
        ]?;

        // Construct the new records
        let mut old_records = vec![];
        for record in records_to_spend {
            old_records.push(record.record);
        }

        let mut old_account_private_keys = vec![];
        for private_key in spenders {
            old_account_private_keys.push(private_key.private_key);
        }

        while old_records.len() < Components::NUM_INPUT_RECORDS {
            let sn_randomness: [u8; 32] = rng.gen();
            let old_sn_nonce = parameters.serial_number_nonce.hash(&sn_randomness)?;

            let private_key = old_account_private_keys[0].clone();
            let address = AccountAddress::<Components>::from_private_key(
                &parameters.account_signature,
                &parameters.account_commitment,
                &parameters.account_encryption,
                &private_key,
            )?;

            let dummy_record = InstantiatedDPC::generate_record(
                &parameters,
                &old_sn_nonce,
                &address,
                true, // The input record is dummy
                0,
                &RecordPayload::default(),
                &noop_program_id,
                &noop_program_id,
                rng,
            )?;

            old_records.push(dummy_record);
            old_account_private_keys.push(private_key);
        }

        assert_eq!(old_records.len(), Components::NUM_INPUT_RECORDS);

        // Enforce that the old record addresses correspond with the private keys
        for (private_key, record) in old_account_private_keys.iter().zip(&old_records) {
            let address = AccountAddress::<Components>::from_private_key(
                &parameters.account_signature,
                &parameters.account_commitment,
                &parameters.account_encryption,
                &private_key,
            )?;

            assert_eq!(&address, record.owner());
        }

        assert_eq!(old_records.len(), Components::NUM_INPUT_RECORDS);
        assert_eq!(old_account_private_keys.len(), Components::NUM_INPUT_RECORDS);

        // Decode new recipient data
        let mut new_record_owners = vec![];
        let mut new_is_dummy_flags = vec![];
        let mut new_values = vec![];
        for (recipient, amount) in recipients.iter().zip(recipient_amounts) {
            new_record_owners.push(recipient.address.clone());
            new_is_dummy_flags.push(false);
            new_values.push(amount);
        }

        // Fill any unused new_record indices with dummy output values
        while new_record_owners.len() < Components::NUM_OUTPUT_RECORDS {
            new_record_owners.push(new_record_owners[0].clone());
            new_is_dummy_flags.push(true);
            new_values.push(0);
        }

        assert_eq!(new_record_owners.len(), Components::NUM_OUTPUT_RECORDS);
        assert_eq!(new_is_dummy_flags.len(), Components::NUM_OUTPUT_RECORDS);
        assert_eq!(new_values.len(), Components::NUM_OUTPUT_RECORDS);

        let new_birth_program_ids = vec![noop_program_id.clone(); Components::NUM_OUTPUT_RECORDS];
        let new_death_program_ids = vec![noop_program_id.clone(); Components::NUM_OUTPUT_RECORDS];
        let new_payloads = vec![RecordPayload::default(); Components::NUM_OUTPUT_RECORDS];

        // Generate a random memo
        let memo = rng.gen();

        // Generate transaction

        // Offline execution to generate a DPC transaction
        let execute_context = <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::execute_offline(
            &parameters,
            &old_records,
            &old_account_private_keys,
            &new_record_owners,
            &new_is_dummy_flags,
            &new_values,
            &new_payloads,
            &new_birth_program_ids,
            &new_death_program_ids,
            &memo,
            network_id,
            rng,
        )?;

        Ok(Self { execute_context })
    }
}

impl FromStr for OfflineTransaction {
    type Err = DPCError;

    fn from_str(execute_context: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            execute_context: ExecuteContext::<Components>::read(&hex::decode(execute_context)?[..])?,
        })
    }
}

impl fmt::Display for OfflineTransaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            hex::encode(to_bytes![self.execute_context].expect("couldn't serialize to bytes"))
        )
    }
}
