# Simple Node

## Quick Start

To start 4 **BFT** nodes, run:
```bash
./start-nodes.sh bft
```

To start 4 **Narwhal** nodes, run:
```bash
./start-nodes.sh narwhal
```

(WIP - not ready) To monitor the nodes, run:
```bash
cargo run --release -- example monitor
```

## Development

To start 4 **BFT** nodes manually, run:
```bash
# Terminal 1
cargo run --release --example simple_node bft 0 4
# Terminal 2
cargo run --release --example simple_node bft 1 4
# Terminal 3
cargo run --release --example simple_node bft 2 4
# Terminal 4
cargo run --release --example simple_node bft 3 4
```

To start 4 **Narwhal** nodes manually, run:
```bash
# Terminal 1
cargo run --release --example simple_node narwhal 0 4
# Terminal 2
cargo run --release --example simple_node narwhal 1 4
# Terminal 3
cargo run --release --example simple_node narwhal 2 4
# Terminal 4
cargo run --release --example simple_node narwhal 3 4
```

These initialize 4 nodes, and tells each node that there are 4 validators in the committee.

## Advanced Usage

You may optionally provide a filename as last argument.
The file must contain the peer node IDs, IP addresses and ports, in the following form `node_id=ip:port`:
```
0=192.168.1.1:5000
1=192.168.1.2:5001
2=192.168.1.3:5002
3=192.168.1.4:5003
```

If this parameter is not present, all nodes are run on localhost.
