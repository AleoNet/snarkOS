<p align="center">
    <img alt="snarkOS" width="1412" src="https://cdn.aleo.org/snarkos/banner.png">
</p>

<p align="center">
    <a href="https://circleci.com/gh/AleoHQ/snarkOS"><img src="https://circleci.com/gh/AleoHQ/snarkOS.svg?style=svg&circle-token=6e9ad6d39d95350544f352d34e0e5c62ef54db26"></a>
    <a href="https://codecov.io/gh/AleoHQ/snarkOS"><img src="https://codecov.io/gh/AleoHQ/snarkOS/branch/master/graph/badge.svg?token=cck8tS9HpO"/></a>
    <a href="https://discord.gg/5v2ynrw2ds"><img src="https://img.shields.io/discord/700454073459015690?logo=discord"/></a>
</p>

## <a name='TableofContents'></a>Table of Contents

* [1. Overview](#1-overview)
* [2. Build Guide](#2-build-guide)
    * [2.1 Requirements](#21-requirements)
    * [2.2 Installation](#22-installation)
* [3a. Run an Aleo Client Node](#3a-run-an-aleo-client-node)
* [3b. Run an Aleo Mining Node](#3a-run-an-aleo-mining-node)
* [4. Testnet2 FAQs](#4-testnet2-faqs)
* [5. Command Line Interface](#5-configuration-file)
* [6. Development Guide](#6-development-guide)
* [7. License](#7-license)

[comment]: <> (* [4. JSON-RPC Interface]&#40;#4-json-rpc-interface&#41;)
[comment]: <> (* [5. Additional Information]&#40;#5-additional-information&#41;)

## 1. Overview

__snarkOS__ is a decentralized operating system for private applications. It forms the backbone of [Aleo](https://aleo.org/) and
enables applications to verify and store state in a publicly verifiable manner.

## 2. Build Guide

Before beginning, please ensure your machine has `Rust v1.56+` installed. Instructions to [install Rust can be found here.](https://www.rust-lang.org/tools/install)

### 2.1 Requirements

The following are **minimum** requirements to run an Aleo node:
 - **CPU**: 16-cores (32-cores preferred)
 - **RAM**: 16GB of memory (32GB preferred)
 - **Storage**: 128GB of disk space
 - **Network**: 50 Mbps of upload **and** download bandwidth

Please note to run an Aleo mining node that is **competitive**, the machine will require more than these requirements.

### 2.2 Installation

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

## 3a. Run an Aleo Client Node

Start by following the instructions in the [Build Guide](#2-build-guide).

Next, to start a client node, from the snarkOS directory, run:
```
./run-client.sh
```

## 3b. Run an Aleo Mining Node

Start by following the instructions in the [Build Guide](#2-build-guide).

Next, to generate an Aleo miner address, run:
```
snarkos experimental new_account
```
or from the snarkOS directory, run:
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

## 4. Testnet2 FAQs

### 1. The node is unable to connect to peers on the network.

- Ensure ports `4132/tcp` and `3032/tcp` are open on your router and OS firewall.
- Ensure snarkOS is started using `run-client.sh` or `run-miner.sh`

## 5. Command Line Interface

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

## 6. Development Guide

In one terminal, start the first node by running:
```
cargo run --release -- --node 4135 --rpc 3035 --miner aleo1d5hg2z3ma00382pngntdp68e74zv54jdxy249qhaujhks9c72yrs33ddah
```

After the first node starts, in a second terminal, run:
```
cargo run --release
```

We welcome all contributions to snarkOS. Please refer to the [license](#7-license) for the terms of contributions.

## 7. License

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](./LICENSE.md)
