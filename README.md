# snarkOS

## Development Guide

In one terminal, start the first node by running:
```
cargo run --release --node 4135 --rpc 3035 --miner aleo1d5hg2z3ma00382pngntdp68e74zv54jdxy249qhaujhks9c72yrs33ddah
```

After the first node starts, in a second terminal, run:
```
cargo run --release --node 4132 --rpc 3032
```

## Usage Guide

### Connecting to Aleo Testnet II

To start a client node, run:
```
snarkos
```

To start a mining node, run:
```
snarkos --is-miner --miner-address {ALEO_ADDRESS}
```

To run a node with custom settings, refer to the full list of options and flags available in the CLI.

### Command Line Interface

Full list of CLI flags and options can be viewed with `snarkos --help`:

```
snarkOS <version>
The Aleo Team <hello@aleo.org>

USAGE:
    snarkos [FLAGS] [OPTIONS]

FLAGS:
    -d, --debug       Enable debug mode
    -h, --help        Prints help information
    -V, --version     Prints version information

OPTIONS:
        --miner <address>    Specify the address that will receive miner rewards
        --node <port>        Specify the port for the node server
        --rpc <port>         Specify the port for the RPC server
        --verbose <verbose>  Specify the verbosity (default = 1) of the node [possible values: 0, 1, 2, 3]
```
