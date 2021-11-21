<p align="center">
    <img alt="snarkOS" width="1412" src="https://cdn.aleo.org/snarkos/banner.png">
</p>

## Build Guide

Before beginning, please ensure your machine has `Rust v1.56+` installed. Instructions to [install Rust can be found here.](https://www.rust-lang.org/tools/install)

### Installation

Start by cloning the snarkOS Github repository:
```
git clone https://github.com/AleoHQ/snarkOS.git --depth 1
```

Next, move into the snarkOS directory:
```
cd snarkOS
```

**[For Ubuntu users]** A helper script to install dependencies is available. From the snarkOS directory, run:
```
./testnet2_ubuntu.sh
```

## Run an Aleo Client Node

To start a client node, from the snarkOS directory, run:
```
./run-client.sh
```

## Run an Aleo Mining Node

To generate an Aleo miner address, run:
```
snarkos experimental new_account
```
or
```
cargo run --release -- experimental new_account
```
This will output a new Aleo account in the terminal.

**Please remember to save the account private key and view key.** The following is an example output:
```
 Attention - Remember to store this account private key and view key.

  Private Key  APrivateKey1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx  <-- Save Me
     View Key  AViewKey1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx  <-- Save Me
      Address  aleo1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx  <-- Use Me For The Next Step
```

Next, to start a mining node, from the snarkOS directory, run:
```
./run-miner.sh
```
When prompted, enter your Aleo miner address:
```
Enter your Aleo miner address:
aleo1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

## Testnet2 FAQs

### 1. The node is unable to connect to peers on the network.

- Ensure ports `4132/tcp` and `3032/tcp` are open on your router and OS firewall.
- Ensure snarkOS is started using `run-client.sh` or `run-miner.sh`

## Command Line Interface

To run a node with custom settings, refer to the full list of options and flags available in the snarkOS CLI.

The full list of CLI flags and options can be viewed with `snarkos --help`:
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
        --verbosity <verbosity>      Specify the verbosity of the node [options: 0, 1, 2, 3] [default: 2]

SUBCOMMANDS:
    help      Prints this message or the help of the given subcommand(s)
    update    Updates snarkOS to the latest version
```

## Development Guide

In one terminal, start the first node by running:
```
cargo run --release -- --node 4135 --rpc 3035 --miner aleo1d5hg2z3ma00382pngntdp68e74zv54jdxy249qhaujhks9c72yrs33ddah
```

After the first node starts, in a second terminal, run:
```
cargo run --release
```
