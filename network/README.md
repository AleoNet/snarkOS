# snarkOS-network

Networking in snarkOS is built using asynchronous rust and [tokio.rs](https://docs.rs/tokio/).
Tokio threads are spawned to handle new connections and send messages back to the main event loop.
The main event loop is started from [server.rs](./src/server/server.rs). 

## modules

### context

Contains network [context](./src/context/context.rs) 
and thread-safe components for managing stateful information such as 
[connections](./src/context/connections.rs), 
[handshakes](./src/context/handshakes.rs), 
[peers](./src/context/peers.rs), and 
[pings](./src/context/pings.rs).

### message

Contains all network message [types](./src/message/types) and serialization standards for reading and writing to asynchronous tcp streams.

### protocol

Contains components for [connecting](./src/protocol/handshake.rs), [maintaining](./src/protocol/ping_protocol.rs), and [syncing](./src/protocol/sync.rs) with remote peers.

### server

Contains components for starting the [miner](src/server/start_miner.rs) and [server](./src/server/server.rs) as well as functions for the [connection handler](./src/server/connection_handler.rs) and [message handler](./src/server/message_handler.rs).

### test_data

Contains helper functions for network unit and integration tests.
