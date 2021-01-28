The snarkOS network protocol establishes a peer-to-peer network of nodes that maintains ledger liveness by actively exchanging transactions and blocks.

## Overview

snarkOS uses TCP connections to facilitate data transfers over the network.
Networking on snarkOS is built with asynchronous calls in Rust and [tokio.rs](https://docs.rs/tokio/).
Tokio tasks are spawned to handle new connections and send messages to the main event loop.

snarkOS downloads, verifies, and stores the history of valid blocks and transactions prior to becoming an active node on the network.

## Peer Discovery

When a node joins the network for the first time, it needs to populate a list of active peers in the network.
In order to bootstrap peer discovery, snarkOS includes a set of optional bootnodes which provides an initial set of peers.
To allow users flexibility, snarkOS provides allows users to configure the initial set of nodes in the configuration file,
or as a input via a command-line flag.

Once a node is connected to one or more nodes, it may scan the network to discover more peers.
This processes starts by asking peers for more connected nodes in the network with a `GetPeers` message,
followed by attempts to establish a connection with each newly discovered peer.

Upon success, snarkOS will store the new peer address to allow it to connect directly with this peer in the future,
without needing to use bootnodes to startup in the future.

#### Bootnodes

Bootnodes operate like other full nodes and serve as a public access point for all peers in the network.
Bootnodes are run by community members and bolster the network
by enabling new nodes to connect and participate in the network effortlessly.

## Connecting to Peers

Peer connections are established with a handshake.
A valid handshake begins with a `Version` message that includes the node's version, block height, and current timestamp.
The receiver returns with its own `Version` message.
Then, both nodes send a `Verack` message acknowledging the receipt of the `Version` message
and establishes a peer connection.

Peer connections are maintained with a ping-pong protocol that periodically relays `Ping` / `Pong` messages to
verify that peers are still connected. snarkOS will update its peer book to account for newly-connected peers,
and disconnected peers.

## Block Download/Sync

Before a node can participate in the network, it must sync itself to the latest state of the ledger.
Whether a node is newly connecting to the network or simply has stale state,
it must sync with its peers, and download its missing blocks and transactions.

snarkOS uses a "Header-First" approach to syncing blocks,
where a node downloads and validates each block header before downloading the corresponding full block, in parallel.

When a node determines it needs to download state, it selects a peer as the sync-node and sends it a `GetSync` message.
The `GetSync` message contains information about the current block state of the node,
so the sync-node is able to determine which block headers are necessary to send as a response.

Upon receiving a `GetSync` message, the sync-node sends back at most 100 block headers via a `Sync` message.
The requester then validates these headers and downloads the blocks in parallel by sending out `GetBlock` messages.
After these blocks have been downloaded, the requester sends another `GetSync` message,
and repeats this process until its chain state is fully up to date.

Here is a basic iteration of the sync protocol:

|   Message  |   Sender  |  Receiver | Data                                |
|:----------:|:---------:|:---------:|-------------------------------------|
| `GetSync`  | Node      | Sync Node | 1 or more block hashes              |
| `Sync`     | Sync Node | Node      | Up to 100 new block headers         |
| `GetBlock` | Node      | Any Peer  | Block header of the requested block |
| `Block`    | Any Peer  | Node      | A serialized block                  |

## Transaction Broadcasting

A node may broadcast a transaction to the network by sending a `Transaction` message to its connected peers.
The peers receiving this transaction verify the transaction
and further propagate the transaction by broadcasting it to its connected peers.
This transaction continues through the network until it is propagated to every connected peer in the network.

## Block Broadcasting

A node may broadcast a block using a `Block` message, in the same manner as broadcasting a transaction.
