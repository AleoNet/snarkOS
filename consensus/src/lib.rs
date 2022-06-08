use std::hash::Hash;

mod block_tree;
mod election;
mod ledger;
mod manager;
mod mempool;
mod message;
mod pacemaker;
mod safety;

// TODO: what should the value of f be?
pub const F: usize = 11;

// TODO: decide on the type; also, Diem only recognizes a single round type
pub type Round = usize;

// TODO: decide on the type; used in the whitepaper, but might be the same as Round
pub type BlockRound = usize;

// TODO: decide on the type; used in the whitepaper, but might be the same as Round
pub type QcRound = usize;

// FIXME: pick a hash function
pub fn hash<T: Hash>(object: &T) -> u64 {
    todo!()
}
