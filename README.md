<p align="center">
    <img alt="snarkOS" width="1412" src="https://cdn.aleo.org/snarkos/banner.png">
</p>

<p align="center">
    <a href="https://circleci.com/gh/AleoHQ/snarkOS"><img src="https://circleci.com/gh/AleoHQ/snarkOS.svg?style=svg&circle-token=6e9ad6d39d95350544f352d34e0e5c62ef54db26"></a>
    <a href="https://codecov.io/gh/AleoHQ/snarkOS"><img src="https://codecov.io/gh/AleoHQ/snarkOS/branch/master/graph/badge.svg?token=cck8tS9HpO"/></a>
    <a href="https://www.aleo.org/discord"><img src="https://img.shields.io/discord/700454073459015690?logo=discord"/></a>
</p>

## <a name='TableofContents'></a>Table of Contents

* [1. Overview](#1-overview)
* [2. Build Guide](#2-build-guide)
  * [2.1 Requirements](#21-requirements)
  * [2.2 Installation](#22-installation)
* [3. Run an Aleo Node](#3-run-an-aleo-node)
  * [3a. Run an Aleo Client](#3a-run-an-aleo-client)
  * [3b. Run an Aleo Prover](#3a-run-an-aleo-prover)
* [4. FAQs](#4-faqs)
* [5. Command Line Interface](#5-configuration-file)
* [6. Development Guide](#6-development-guide)
  * [6.1 Quick Start](#61-quick-start)
  * [6.2 Operations](#61-operations)
* [7. License](#7-license)

[comment]: <> (* [4. JSON-RPC Interface]&#40;#4-json-rpc-interface&#41;)
[comment]: <> (* [5. Additional Information]&#40;#5-additional-information&#41;)

## 1. Overview

__snarkOS__ is a decentralized operating system for zero-knowledge applications.
This code forms the backbone of [Aleo](https://aleo.org/) network,
which verifies transactions and stores the encrypted state applications in a publicly-verifiable manner.

## 2. Build Guide

### 2.1 Requirements

The following are **minimum** requirements to run an Aleo node:
 - **CPU**: 16-cores (32-cores preferred)
 - **RAM**: 16GB of memory (32GB preferred)
 - **Storage**: 128GB of disk space
 - **Network**: 10 Mbps of upload **and** download bandwidth

Please note to run an Aleo Prover that is **competitive**, the machine will require more than these requirements.

### 2.2 Installation

Before beginning, please ensure your machine has `Rust v1.65+` installed. Instructions to [install Rust can be found here.](https://www.rust-lang.org/tools/install)

Start by cloning this Github repository:
```
git clone https://github.com/AleoHQ/snarkOS.git --depth 1
```

Next, move into the `snarkOS` directory:
```
cd snarkOS
```

**[For Ubuntu users]** A helper script to install dependencies is available. From the `snarkOS` directory, run:
```
./build_ubuntu.sh
```

Lastly, install `snarkOS`:
```
cargo install --path .
```

## 3. Run an Aleo Node

## 3a. Run an Aleo Client

Start by following the instructions in the [Build Guide](#2-build-guide).

Next, to start a client node, from the `snarkOS` directory, run:
```
./run-client.sh
```

## 3b. Run an Aleo Prover

Start by following the instructions in the [Build Guide](#2-build-guide).

Next, generate an Aleo account address:
```
snarkos account new
```
This will output a new Aleo account in the terminal.

**Please remember to save the account private key and view key.** The following is an example output:
```
 Attention - Remember to store this account private key and view key.

  Private Key  APrivateKey1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx  <-- Save Me And Use In The Next Step
     View Key  AViewKey1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx  <-- Save Me
      Address  aleo1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx  <-- Save Me
```

Next, to start a proving node, from the `snarkOS` directory, run:
```
./run-prover.sh
```
When prompted, enter your Aleo private key:
```
Enter the Aleo Prover account private key:
APrivateKey1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

## 4. FAQs

### 1. My node is unable to compile.

- Ensure your machine has `Rust v1.65+` installed. Instructions to [install Rust can be found here.](https://www.rust-lang.org/tools/install)
- If large errors appear during compilation, try running `cargo clean`.
- Ensure `snarkOS` is started using `./run-client.sh` or `./run-prover.sh`.

### 2. My node is unable to connect to peers on the network.

- Ensure ports `4133/tcp` and `3033/tcp` are open on your router and OS firewall.
- Ensure `snarkOS` is started using `./run-client.sh` or `./run-prover.sh`.

### 3. I can't generate a new address ### 

- Before running the command above (`snarkos account new`) try `source ~/.bashrc`
- Also double-check the spelling of `snarkos`. Note the directory is `/snarkOS`, the command is `snarkos`

## 5. Command Line Interface

To run a node with custom settings, refer to the full list of options and flags available in the `snarkOS` CLI.

The full list of CLI flags and options can be viewed with `snarkos --help`:
```
snarkOS 
The Aleo Team <hello@aleo.org>

USAGE:
    snarkos [OPTIONS] <SUBCOMMAND>

OPTIONS:
    -h, --help                     Print help information
    -v, --verbosity <VERBOSITY>    Specify the verbosity [options: 0, 1, 2, 3] [default: 2]

SUBCOMMANDS:
    account    Commands to manage Aleo accounts
    clean      Cleans the snarkOS node storage
    help       Print this message or the help of the given subcommand(s)
    start      Starts the snarkOS node
    update     Update snarkOS
```

The following are the options for the `snarkos start` command:
```
snarkos-start 
Starts the snarkOS node

USAGE:
    snarkos start [OPTIONS]

OPTIONS:
        --beacon <BEACON>          Specify this as a beacon, with the given account private key for this node
        --client <CLIENT>          Specify this as a client, with an optional account private key for this node
        --connect <CONNECT>        Specify the IP address and port of a peer to connect to [default: ]
        --dev <DEV>                Enables development mode, specify a unique ID for this node
    -h, --help                     Print help information
        --logfile <LOGFILE>        Specify the path to the file where logs will be stored [default: /tmp/snarkos.log]
        --network <NETWORK>        Specify the network of this node [default: 3]
        --node <NODE>              Specify the IP address and port for the node server [default: 0.0.0.0:4133]
        --nodisplay                If the flag is set, the node will not render the display
        --norest                   If the flag is set, the node will not initialize the REST server
        --prover <PROVER>          Specify this as a prover, with the given account private key for this node
        --rest <REST>              Specify the IP address and port for the REST server [default: 0.0.0.0:3033]
        --validator <VALIDATOR>    Specify this as a validator, with the given account private key for this node
        --verbosity <VERBOSITY>    Specify the verbosity of the node [options: 0, 1, 2, 3] [default: 2]
```

## 6. Development

### 6.1 Quick Start

In one terminal, start the beacon by running:
```
cargo run --release -- start --nodisplay --dev 0 --beacon ""
```

In a second terminal, run:
```
cargo run --release -- start --nodisplay --dev 1 --prover ""
```

This procedure can be repeated to start more nodes.

### 6.2 Operations

It is important to initialize the nodes starting from `0` and incrementing by `1` for each new node.

The following is a list of options to initialize a node (replace `XX` with a number starting from `0`):
```
cargo run --release -- start --nodisplay --dev XX --beacon ""
cargo run --release -- start --nodisplay --dev XX --validator ""
cargo run --release -- start --nodisplay --dev XX --prover ""
cargo run --release -- start --nodisplay --dev XX --client ""
cargo run --release -- start --nodisplay --dev XX
```

When no node type is specified, the node will default to `--client`.

##### Clean Up

To clean up the node storage, run:
```
cargo run --release -- clean --dev XX
```

## 7. License

We welcome all contributions to `snarkOS`. Please refer to the [license](#7-license) for the terms of contributions.

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](./LICENSE.md)
