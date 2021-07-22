# Storage

## Trait Model

### Storage

snarkOS storage is exposed exclusively through the [Storage](https://github.com/AleoHQ/snarkOS/blob/staging/storage/src/storage.rs) trait to other components of snarkOS. This trait defines application-level operations that are guaranteed to be atomic and strongly consistent/ordered.

### KeyValueStorage

Currently, snarkOS exclusively supports RocksDB for persistent storage, and exposes general key-value storage functionality through the [KeyValueStorage](https://github.com/AleoHQ/snarkOS/blob/staging/storage/src/key_value/mod.rs) trait.

Implementors of this trait can define the basic operations of a key value store, then be used as a `Storage` implemenation with the established semantics for RocksDB through the [KeyValueStore](https://github.com/AleoHQ/snarkOS/blob/staging/storage/src/key_value/storage.rs)

See [RocksDB](https://github.com/AleoHQ/snarkOS/blob/staging/storage/src/rocks.rs) for example implementation.

#### Agent
The `KeyValueStore` guarantees atomicity and strong consistency/ordering in the underlying storage implementation by using an actor model [Agent](https://github.com/AleoHQ/snarkOS/blob/staging/storage/src/key_value/agent/mod.rs) to execute each read or write operation.

## Dividing Storage and Consensus

There is a good amount of haziness between where consensus ends and storage begins. In large part, this is due to needing to tightly couple some consensus semantics with hard DB implementations for performance. The result is functions like `Storage::get_block_locator_hashes`.

As a standard, we are drawing the line where there is a large series of storage-only calls for one atomic, application-level operation. I.e. committing a block, inserting a block, getting locator hashes, checking canon state, are all examples of storage level operations. Checking block validity, integrating with ledger/memory pool, and checking transaction validity are not storage-heavy operations, so are left in consensus.
