#![deny(unused_import_braces, unused_qualifications, trivial_casts, trivial_numeric_casts)]
#![deny(unused_qualifications, variant_size_differences, stable_features, unreachable_pub)]
#![deny(non_shorthand_field_patterns, unused_attributes, unused_extern_crates)]
#![deny(
    renamed_and_removed_lints,
    stable_features,
    unused_allocation,
    unused_comparisons,
    bare_trait_objects
)]
#![deny(
    const_err,
    unused_must_use,
    unused_mut,
    unused_unsafe,
    private_in_public,
    unsafe_code
)]
#![forbid(unsafe_code)]

pub mod consensus;
pub use consensus::*;

pub mod difficulty;
pub use difficulty::*;

pub mod miner;
pub use miner::Miner;

pub mod memory_pool;
pub use memory_pool::MemoryPool;

use snarkos_dpc::base_dpc::instantiated::{CommitmentMerkleParameters, Tx};
use snarkos_storage::Ledger;

pub type MerkleTreeLedger = Ledger<Tx, CommitmentMerkleParameters>;
