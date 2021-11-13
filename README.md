<p align="center">
    <img alt="snarkOS" width="1412" src="https://cdn.aleo.org/snarkos/banner.png">
</p>

## Development Guide

In one terminal, start the first node by running:
```
cargo run --release -- --node 4135 --rpc 3035 --miner aleo1d5hg2z3ma00382pngntdp68e74zv54jdxy249qhaujhks9c72yrs33ddah
```

After the first node starts, in a second terminal, run:
```
cargo run --release
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
        --display    If the flag is set, the node will render a read-only display
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --miner <miner>              Specify this as a mining node, with the given miner address
    -n, --network <network>          Specify the network of this node [default: 2]
        --node <node>                Specify the port for the node server
        --rpc <rpc>                  Specify the port for the RPC server
        --username <rpc-username>    Specify the username for the RPC server [default: root]
        --password <rpc-password>    Specify the password for the RPC server [default: pass]
        --verbosity <verbosity>      Specify the verbosity of the node [options: 0, 1, 2, 3] [default: 3]

SUBCOMMANDS:
    help      Prints this message or the help of the given subcommand(s)
    update    Updates snarkOS to the latest version
```
