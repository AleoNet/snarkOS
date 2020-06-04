use snarkos_consensus::ConsensusParameters;
use snarkos_dpc::base_dpc::instantiated::*;

mod e2e;
pub use e2e::*;

mod fixture;
pub use fixture::*;

pub const TEST_CONSENSUS: ConsensusParameters = ConsensusParameters {
    max_block_size: 1_000_000usize,
    max_nonce: u32::max_value(),
    target_block_time: 2i64, //unix seconds
};
