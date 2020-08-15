use snarkos_consensus::ConsensusParameters;
use snarkos_errors::objects::TransactionError;
use snarkos_models::objects::Transaction;
use snarkos_objects::Network;
use snarkos_posw::PoswMarlin;
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use once_cell::sync::Lazy;
use std::io::{Read, Result as IoResult, Write};

mod e2e;
pub use e2e::*;

mod fixture;
pub use fixture::*;

pub static TEST_CONSENSUS: Lazy<ConsensusParameters> = Lazy::new(|| ConsensusParameters {
    max_block_size: 1_000_000usize,
    max_nonce: u32::max_value(),
    target_block_time: 2i64, //unix seconds
    network: Network::Mainnet,
    verifier: PoswMarlin::verify_only().unwrap(),
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestTx;

impl Transaction for TestTx {
    type Commitment = [u8; 32];
    type Digest = [u8; 32];
    type EncryptedRecord = [u8; 32];
    type LocalDataRoot = [u8; 32];
    type Memorandum = [u8; 32];
    type ProgramCommitment = [u8; 32];
    type SerialNumber = [u8; 32];
    type ValueBalance = i64;

    fn transaction_id(&self) -> Result<[u8; 32], TransactionError> {
        Ok([0u8; 32])
    }

    fn network_id(&self) -> u8 {
        0
    }

    fn ledger_digest(&self) -> &Self::Digest {
        &[0u8; 32]
    }

    fn old_serial_numbers(&self) -> &[Self::SerialNumber] {
        &[[0u8; 32]]
    }

    fn new_commitments(&self) -> &[Self::Commitment] {
        &[[0u8; 32]]
    }

    fn program_commitment(&self) -> &Self::ProgramCommitment {
        &[0u8; 32]
    }

    fn local_data_root(&self) -> &Self::LocalDataRoot {
        &[0u8; 32]
    }

    fn value_balance(&self) -> i64 {
        0
    }

    fn memorandum(&self) -> &Self::Memorandum {
        &[0u8; 32]
    }

    fn encrypted_records(&self) -> &[Self::EncryptedRecord] {
        &[[0u8; 32]]
    }

    fn size(&self) -> usize {
        0
    }
}

impl ToBytes for TestTx {
    #[inline]
    fn write<W: Write>(&self, mut _writer: W) -> IoResult<()> {
        Ok(())
    }
}

impl FromBytes for TestTx {
    #[inline]
    fn read<R: Read>(mut _reader: R) -> IoResult<Self> {
        Ok(Self)
    }
}
