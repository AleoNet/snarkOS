# Simple Node

To start 4 nodes, run:
```bash
# Terminal 1
cargo run --example simple_node 0 4
# Terminal 2
cargo run --example simple_node 1 4
# Terminal 3
cargo run --example simple_node 2 4
# Terminal 4
cargo run --example simple_node 3 4
```
This initializes 4 nodes, and tells each node that there are 4 validators in the committee.
