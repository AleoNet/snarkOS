# Consensus

## Loading
1. A `MerkleLedger` is initialized and owned exclusively by [Consensus](https://github.com/AleoHQ/snarkOS/blob/staging/consensus/src/consensus/mod.rs).
2. `Storage` is asked for all commitments, serial numbers and memos of transactions and all past ledger digests and sent to the owned `MerkleLedger`.
3. If the configured genesis block isn't committed, it is immediately committed.
4. The tip of the current canon chain, according to `Storage`, is checked for descendents and if any are found, they are committed.

## Interation with Storage
A `Storage` handle is owned by `Consensus` -- it *should* be the same as the general node, but could have a different caching layer/etc.

Calls to storage will be made throughout the `Consensus`'s lfietime; it relies heavily on the atomicity of storage calls to maintain consistency.

## Interaction with Ledger

The instantiated `Ledger` is exclusively owned by `Consensus` behind an actor model. Consensus owns the task of keeping `Ledger` and `Storage` in sync as the set of commitments, serial numbers, etc are changed via commit and decommit operations.

## Actor Model

The `ConsensusInner` is owned by a task spawned during `Consensus` initialization, which is dropped when all `Consensus` handles are dropped. It concretely holds the `Ledger` and `MemoryPool` and provides application-level strongly consistent order and atomicity over the synchronization between `Storage` and `Ledger`.
