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
Usage: simple_node [OPTIONS] --mode <MODE> --id <ID> --num-nodes <N>

Options:
      --mode <MODE>
          The mode to run the node in

          Possible values:
          - narwhal: Runs the node with the Narwhal memory pool protocol
          - bft:     Runs the node with the Bullshark BFT protocol (on top of Narwhal)

      --id <ID>
          The ID of the node

      --num-nodes <N>
          The number of nodes in the network

      --config <PATH>
          If set, the path to the file containing the committee configuration

      --fire-solutions [<INTERVAL_MS>]
          Enables the solution cannons, and optionally the interval in ms to run them on

      --fire-transactions [<INTERVAL_MS>]
          Enables the transaction cannons, and optionally the interval in ms to run them on

      --fire-transmissions [<INTERVAL_MS>]
          Enables the solution and transaction cannons, and optionally the interval in ms to run them on

  -h, --help
          Print help (see a summary with '-h')
```

To start 4 **BFT** nodes manually, run:
```bash
# Terminal 1
cargo run --release --example simple_node --mode bft --id 0 --num-nodes 4 --fire-transmissions
# Terminal 2
cargo run --release --example simple_node --mode bft --id 1 --num-nodes 4 --fire-transmissions
# Terminal 3
cargo run --release --example simple_node --mode bft --id 2 --num-nodes 4 --fire-transmissions
# Terminal 4
cargo run --release --example simple_node --mode bft --id 3 --num-nodes 4 --fire-transmissions
```

To start 4 **Narwhal** nodes manually, run:
```bash
# Terminal 1
cargo run --release --example simple_node --mode narwhal --id 0 --num-nodes 4 --fire-transmissions
# Terminal 2
cargo run --release --example simple_node --mode narwhal --id 1 --num-nodes 4 --fire-transmissions
# Terminal 3
cargo run --release --example simple_node --mode narwhal --id 2 --num-nodes 4 --fire-transmissions
# Terminal 4
cargo run --release --example simple_node --mode narwhal --id 3 --num-nodes 4 --fire-transmissions
```

These initialize 4 nodes, and tells each node that there are 4 validators in the committee.

## Advanced Usage

You may optionally provide a filename as an option with `--config`.
The file must contain the peer node IDs, IP addresses and ports, in the following form `id=ip:port`:
```
0=192.168.1.1:5000
1=192.168.1.2:5001
2=192.168.1.3:5002
3=192.168.1.4:5003
```

If this parameter is not present, all nodes are run on localhost.

In addition, `--fire-transmissions` will enable the transaction and solution cannons for each node.
If enabled, the interval in milliseconds can optionally be passed in as an argument.
