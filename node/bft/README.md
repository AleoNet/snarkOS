# snarkos-node-bft

[![Crates.io](https://img.shields.io/crates/v/snarkos-node-bft.svg?color=neon)](https://crates.io/crates/snarkos-node-bft)
[![Authors](https://img.shields.io/badge/authors-Aleo-orange.svg)](https://aleo.org)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](./LICENSE.md)

The `snarkos-node-bft` crate provides a node implementation for a BFT-based memory pool.

## Primary

The primary is the coordinator, responsible for advancing rounds and broadcasting the anchor.

#### Triggering Round Advancement

Each round runs until one of two conditions is met:
1. The coinbase target has been reached, or
2. The round has reached its timeout (currently set to 10 seconds)

#### Advancing Rounds

As described in the paper, the BFT advances rounds whenever n − f vertices are delivered.
```
The problem in advancing rounds whenever n − f vertices are delivered is that parties
might not vote for the anchor even if the party that broadcast it is just slightly slower
than the fastest n − f parties. To deal with this, the BFT integrates timeouts into
the DAG construction. If the first n − f vertices a party p gets in an even-numbered round r 
do not include the anchor of round r, then p sets a timer and waits for the anchor
until the timer expires. Similarly, in an odd-numbered round, parties wait for either
f + 1 vertices that vote for the anchor, or 2f + 1 vertices that do not, or a timeout.
```

## Workers

The workers are simple entry replicators that receive transactions from the network and append them to their memory pool.

In order to function properly, workers must be synced to the latest round, and capable of performing verification
on the entries they receive from other validators' workers.

## Test Cases

- Two validators, one with X workers, another with Y workers. Check that they are compatible.
- If a primary sees that f+1 other primaries have certified this round, it should skip to the next round if it has not been certified yet.
- Ensure taking a set number of transmissions from workers leaves the remaining transmissions in place for the next round.
- Send back a mismatching transmission for a transmission ID, ensure it catches it.
- Send back a mismatching certificate for a certificate ID, ensure it catches it.

## Open Questions

1. How does one guarantee the number of accepted transactions and solutions does not exceed the block limits?
   - We need to set limits on the number of transmissions for the workers, but also the primary.
