# snarkOS

## Run

In one terminal, run:
```
cargo run --release --is-miner -p 4135 --rpc-port 3035
```

In another terminal, run:
```
cargo run --release -p 4132 --rpc-port 3032
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
        --is-miner    Start mining blocks from this node
    -V, --version     Prints version information

OPTIONS:
        --miner-address <miner-address>    Specify the address that will receive miner rewards
    -p, --port <port>                      Specify the port the node is run on
        --rpc-port <rpc-port>              Specify the port the json rpc server is run on
        --verbose <verbose>                Specify the verbosity (default = 1) of the node [possible values: 0, 1, 2, 3]
```
