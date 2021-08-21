# Ledger

## Types

### Ledger

The [Ledger](https://github.com/AleoHQ/snarkOS/blob/staging/consensus/src/ledger/mod.rs) trait represents the underlying (presumably merkle tree) implementation for validating transaction commitments, serial numbers, memos, and ledger digests.

### MerkleLedger

The [MerkleLedger](https://github.com/AleoHQ/snarkOS/blob/staging/consensus/src/ledger/merkle.rs) is the current sole implementor of `Ledger` and uses `IndexedMerkleTree` and `IndexedDigests` to provide functionality.

### IndexedDigests

[IndexedDigests](https://github.com/AleoHQ/snarkOS/blob/staging/consensus/src/ledger/indexed_digests.rs) is a wrapper over `indexmap::IndexSet<Digest>`. It provides consistently-ordered O(log n) insertions and O(1) lookups along with checked element removal.

### IndexedMerkleTree

[IndexedMerkleTree](https://github.com/AleoHQ/snarkOS/blob/staging/consensus/src/ledger/indexed_merkle_tree.rs) provides a wrapper over `IndexedDigests` and snarkVM's [MerkleTree](https://github.com/AleoHQ/snarkVM/blob/snarkvm-ir/algorithms/src/merkle_tree/merkle_tree.rs)
