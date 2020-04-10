use snarkos_errors::storage::StorageError;

use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

pub const COL_META: u32 = 0;
pub const COL_BLOCK: u32 = 1;
pub const COL_BLOCK_HASHES: u32 = 2;
pub const COL_BLOCK_NUMBERS: u32 = 3;
pub const COL_BLOCK_TRANSACTIONS: u32 = 4;
pub const COL_TRANSACTIONS: u32 = 5;
pub const COL_TRANSACTION_META: u32 = 6;
pub const COL_CHILD_HASHES: u32 = 7;

pub const NUM_COLS: u32 = 8;

pub const KEY_BEST_BLOCK_NUMBER: &str = "BEST_BLOCK_NUMBER";
pub const KEY_MEMORY_POOL: &str = "MEMORY_POOL";
pub const KEY_PEER_BOOK: &str = "PEER_BOOK";
//pub const KEY_BEST_BLOCK_HASH: &'static str = "BEST_BLOCK_HASH";

/// Batched transaction of database operations.
#[derive(Default, Clone, PartialEq)]
pub struct DatabaseTransaction(pub Vec<Op>);

/// Database operation.
#[derive(Clone, PartialEq)]
pub enum Op {
    Insert { col: u32, key: Vec<u8>, value: Vec<u8> },
    Delete { col: u32, key: Vec<u8> },
}
//
//impl Op {
//    pub fn key(&self) -> &[u8] {
//        match *self {
//            Op::Insert(_, key, _) => &key,
//            Op::Delete(_, key) => &key,
//        }
//    }
//
//    pub fn col(&self) -> u32 {
//        match *self {
//            Op::Insert(col, _, _) => col,
//            Op::Delete(col, _) => col,
//        }
//    }
//}

pub fn bytes_to_u32(bytes: Vec<u8>) -> u32 {
    let mut num_bytes = [0u8; 4];
    num_bytes.copy_from_slice(&bytes);

    u32::from_le_bytes(num_bytes)
}
