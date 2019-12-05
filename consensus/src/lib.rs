#![warn(unused_extern_crates)]
#![forbid(unsafe_code)]

#[macro_use]
extern crate log;

pub mod consensus;
pub use self::consensus::*;

pub mod difficulty;
pub use self::difficulty::*;

pub mod miner;

pub mod verify_transaction;
pub use self::verify_transaction::*;

#[allow(dead_code)]
pub mod test_data;
