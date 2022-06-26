mod block_tree;
mod ledger;
pub mod message;
pub mod validator;

use std::hash::Hash;

pub type N = snarkvm::console::network::Testnet3;
pub type Address = snarkvm::console::account::Address<N>;
pub type Signature = snarkvm::console::account::Signature<N>;

pub const F: usize = 11;

/// This value defines the number of rounds that have taken place since genesis,
/// and includes both successful and timed out rounds.
pub type Round = u64;

// FIXME: pick a hash function
pub fn hash<T: Hash>(object: &T) -> u64 {
    todo!()
}
