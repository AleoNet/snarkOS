use snarkos_consensus::ConsensusParameters;
use snarkos_posw::Posw;

use once_cell::sync::Lazy;

mod e2e;
pub use e2e::*;

mod fixture;
pub use fixture::*;

pub static TEST_CONSENSUS: Lazy<ConsensusParameters> = Lazy::new(|| ConsensusParameters {
    max_block_size: 1_000_000usize,
    max_nonce: u32::max_value(),
    target_block_time: 2i64, //unix seconds
    verifier: Posw::verify_only().unwrap(),
});
