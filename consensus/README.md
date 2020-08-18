# snarkos-consensus

snarkOS-consensus defines the consensus mechanisms of the Aleo network - how blocks are validated, 
block rewards are issued, block times/difficulties are established, and how blocks are mined. 

## Blocks

## Block Difficulty and Block Times 

The block time is the amount of time it takes for the network to produce a valid block.
This block time is variable and based on the network's hashrate, but regulated by the block difficulty. 
The block difficulty is adjusted according to the most recent block times in order to regulate and
stabilize the average block time of the network.

### Block Rewards

A block reward is the total amount of Aleo credits rewarded to the address that mined a block.
This value is the base block reward in addition to the fees paid by all transactions included in the block.

|      Block Number     |   Reward  |
|:---------------------:|:---------:|
| 0 - 3,503,999         | 150 ALEO  |
| 3,504,000 - 7,007,999 | 75 ALEO   |
| 7,008,000 - ∞         | 37.5 ALEO |

Initially, each Aleo block reward is worth 150 Aleo credits. This block reward is halved after every 3,504,000 blocks, which
is approximately four years at an estimated 100 blocks per hour. After two iterations of halving the block reward, it will
remain at 37.5 for perpetuity.

### Verification

Block validation is the process in which the consensus checks that a block is valid in the ledger. A block is valid 
if all the transactions in the block are valid, the total value balance of the block transactions is correct, 
there are no double spends, and the block header attributes are 
valid - timestamp, nonce, PoSW proof, merkle root hash, difficulty target, etc.

## Memory Pool

The memory pool is an in-memory storage of pending transactions that the miner can use to construct blocks.

## Miner

The miner is a CPU implementation of an Aleo miner that fetches transactions from the memory pool and 
attempts to compute a valid nonce for solving a [Proof of Succinct Work](../posw/documentation/) puzzle.

Upon successfully finding a valid block, miners are compensated with a [block reward](#block-rewards) for their contribution.