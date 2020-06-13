#![warn(unused_extern_crates)]
#![forbid(unsafe_code)]

pub mod consensus;
pub use self::consensus::*;

pub mod difficulty;
pub use self::difficulty::*;

pub mod miner;
pub use miner::Miner;

pub mod memory_pool;
pub use memory_pool::MemoryPool;

use snarkos_dpc::base_dpc::instantiated::{CommitmentMerkleParameters, Tx};
use snarkos_storage::Ledger;

pub type MerkleTreeLedger = Ledger<Tx, CommitmentMerkleParameters>;
