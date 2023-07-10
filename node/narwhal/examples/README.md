# Simple Node

## Quick Start

To start 4 **Bullshark** nodes, run:
```bash
./start-nodes.sh bullshark
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

To start 4 **Bullshark** nodes manually, run:
```bash
# Terminal 1
cargo run --release --example simple_node bullshark 0 4
# Terminal 2
cargo run --release --example simple_node bullshark 1 4
# Terminal 3
cargo run --release --example simple_node bullshark 2 4
# Terminal 4
cargo run --release --example simple_node bullshark 3 4
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
