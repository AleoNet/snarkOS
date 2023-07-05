# Simple Node

## Quick Start

To start 4 nodes, run:
```bash
./start-nodes.sh
```

(WIP - not ready) To monitor the nodes, run:
```bash
cargo run --release -- example monitor
```

## Development

To start 4 nodes manually, run:
```bash
# Terminal 1
cargo run --release --example simple_node 0 4
# Terminal 2
cargo run --release --example simple_node 1 4
# Terminal 3
cargo run --release --example simple_node 2 4
# Terminal 4
cargo run --release --example simple_node 3 4
```
This initializes 4 nodes, and tells each node that there are 4 validators in the committee.
