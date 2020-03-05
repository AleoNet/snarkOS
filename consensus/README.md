# snarkOS-consensus

This node is currently configured to add blocks to a proof of work blockchain where the canonical chain is the longest.

## modules

### miner

Contains the memory pool of transactions and the miner to run proof of work and find blocks.

### consensus

Contains methods for calculating proof of work and adding blocks to storage. 
Determines if blocks received by the server are canonical or side chain.

### difficulty

Contains methods for calculating the next block's difficulty based off the time it took the find the last one.

### verify_transaction

Checks coinbase transaction and verifies all transactions for double spends.

### test_data

Contains helper functions for network unit and integration tests.
