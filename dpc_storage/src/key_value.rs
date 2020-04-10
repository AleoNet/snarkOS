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

/// Database operation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Op {
    Insert { col: u32, key: Vec<u8>, value: Vec<u8> },
    Delete { col: u32, key: Vec<u8> },
}

impl Op {
    pub fn key(&self) -> &[u8] {
        match self {
            Op::Insert { key, .. } => &key,
            Op::Delete { key, .. } => &key,
        }
    }

    pub fn col(&self) -> u32 {
        match self {
            Op::Insert { col, .. } => *col,
            Op::Delete { col, .. } => *col,
        }
    }
}

/// Batched transaction of database operations.
#[derive(Default, Clone, PartialEq)]
pub struct DatabaseTransaction(pub Vec<Op>);

impl DatabaseTransaction {
    /// Create new transaction.
    pub fn new() -> Self {
        Self(vec![])
    }

    /// Add a key value pair under a specific col.
    pub fn add(&mut self, col: u32, key: &[u8], value: &[u8]) {
        self.0.push(Op::Insert {
            col,
            key: key.to_vec(),
            value: value.to_vec(),
        })
    }

    /// Delete a value given a col and key.
    pub fn delete(&mut self, col: u32, key: &[u8]) {
        self.0.push(Op::Delete { col, key: key.to_vec() })
    }
}

pub fn bytes_to_u32(bytes: Vec<u8>) -> u32 {
    let mut num_bytes = [0u8; 4];
    num_bytes.copy_from_slice(&bytes);

    u32::from_le_bytes(num_bytes)
}
