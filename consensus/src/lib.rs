use std::hash::Hash;

mod block_tree;
mod election;
mod ledger;
pub mod manager;
mod mempool;
pub mod message;
mod pacemaker;
mod safety;

use snarkvm::console::account::Address;

// TODO: what should the value of f be?
pub const F: usize = 11;

/// This value defines the number of rounds that have taken place since genesis,
/// and includes both successful and timed out rounds.
pub type Round = u64;

// FIXME: pick a hash function
pub fn hash<T: Hash>(object: &T) -> u64 {
    todo!()
}
