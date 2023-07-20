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

```
Usage: simple_node [OPTIONS] --mode <MODE> --node-id <ID> --num-nodes <N>

Options:
      --mode <MODE>
          The mode to run the node in

          Possible values:
          - narwhal: Runs the node with the Narwhal memory pool protocol
          - bft:     Runs the node with the Bullshark BFT protocol (on top of Narwhal)

      --node-id <ID>
          The ID of the node

      --num-nodes <N>
          The number of nodes in the network

      --config <PATH>
          If set, the path to the file containing the committee configuration

      --fire-cannons [<INTERVAL_MS>]
          Enables the tx and solution cannons, and optionally the interval in ms to run them on

  -h, --help
          Print help (see a summary with '-h')
```

To start 4 **BFT** nodes manually, run:
```bash
# Terminal 1
cargo run --release --example simple_node --mode bft --node-id 0 --num-nodes 4 --fire-cannons
# Terminal 2
cargo run --release --example simple_node --mode bft --node-id 1 --num-nodes 4 --fire-cannons
# Terminal 3
cargo run --release --example simple_node --mode bft --node-id 2 --num-nodes 4 --fire-cannons
# Terminal 4
cargo run --release --example simple_node --mode bft --node-id 3 --num-nodes 4 --fire-cannons
```

To start 4 **Narwhal** nodes manually, run:
```bash
# Terminal 1
cargo run --release --example simple_node --mode narwhal --node-id 0 --num-nodes 4 --fire-cannons
# Terminal 2
cargo run --release --example simple_node --mode narwhal --node-id 1 --num-nodes 4 --fire-cannons
# Terminal 3
cargo run --release --example simple_node --mode narwhal --node-id 2 --num-nodes 4 --fire-cannons
# Terminal 4
cargo run --release --example simple_node --mode narwhal --node-id 3 --num-nodes 4 --fire-cannons
```

These initialize 4 nodes, and tells each node that there are 4 validators in the committee.

## Advanced Usage

You may optionally provide a filename as an option with `--config`.
The file must contain the peer node IDs, IP addresses and ports, in the following form `node_id=ip:port`:
```
0=192.168.1.1:5000
1=192.168.1.2:5001
2=192.168.1.3:5002
3=192.168.1.4:5003
```

If this parameter is not present, all nodes are run on localhost.

In addition, `--fire-cannons` will enable the transaction and solution cannons for each node.
If enabled, the interval in milliseconds can optionally be passed in as an argument.
