# snarkOS

## Run

In one terminal, run:
```
cargo run --release 4132 3032
```

In another terminal, run:
```
cargo run --release 4135 3035
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
Run an Aleo node (include -h for more options)

USAGE:
    snarkos [FLAGS] [OPTIONS]

FLAGS:
    -h, --help           Prints help information
        --is-miner       Start mining blocks from this node

OPTIONS:
        --miner-address <miner-address>          Specify the address that will receive miner rewards
    -p, --port <port>                            Specify the port the node is run on
        --rpc-port <rpc-port>                    Specify the port the json rpc server is run on
        --verbose <verbose>                      Specify the verbosity (default = 1) of the node [possible values: 0, 1, 2, 3]
```
