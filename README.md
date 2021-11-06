# snarkOS

## Development Guide

In one terminal, start the first node by running:
```
cargo run --release -- --nodisplay --node 4135 --rpc 3035 --miner aleo1d5hg2z3ma00382pngntdp68e74zv54jdxy249qhaujhks9c72yrs33ddah
```

After the first node starts, in a second terminal, run:
```
cargo run --release -- --nodisplay --node 4132 --rpc 3032
```

## Usage Guide

### Connecting to Aleo Testnet II

To start a client node, run:
```
snarkos
```

To start a mining node, run:
```
snarkos --miner {ALEO_ADDRESS}
```

To run a node with custom settings, refer to the full list of options and flags available in the CLI.

### Command Line Interface

Full list of CLI flags and options can be viewed with `snarkos --help`:

```
snarkos
The Aleo Team <hello@aleo.org>

USAGE:
    snarkos [FLAGS] [OPTIONS]

FLAGS:
    -h, --help         Prints help information
    -n, --nodisplay    If the flag is set, the node will only output logs
    -V, --version      Prints version information

OPTIONS:
        --miner <miner>            Specify this as a mining node, with the given miner address
    -n, --network <network>        Specify the network of this node [default: 2]
        --node <node>              Specify the port for the node server
        --rpc <rpc>                Specify the port for the RPC server
        --verbosity <verbosity>    Specify the verbosity of the node [options: 0, 1, 2, 3] [default: 3]
```
