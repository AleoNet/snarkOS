<h1 align="center">snarkOS</h1>

<p align="center">
    <a href="https://circleci.com/gh/AleoHQ/snarkOS"><img src="https://circleci.com/gh/AleoHQ/snarkOS.svg?style=svg&circle-token=6e9ad6d39d95350544f352d34e0e5c62ef54db26"></a>
    <a href="https://codecov.io/gh/AleoHQ/snarkOS"><img src="https://codecov.io/gh/AleoHQ/snarkOS/branch/master/graph/badge.svg?token=cck8tS9HpO"/></a>
    <a href="https://discord.gg/6WG7Bck"><img src="https://img.shields.io/discord/700454073459015690?logo=discord"/></a>
</p>

## <a name='TableofContents'></a>Table of Contents

* [1. Overview](#1-overview)
* [2. Build Guide](#2-build-guide)
    * [2.1 Install Rust](#21-install-rust)
    * [2.2a Build from Crates.io](#22a-build-from-cratesio)
    * [2.2b Build from Source Code](#22b-build-from-source-code)
    * [2.2c Build with Docker](#22c-build-with-docker)
* [3. Usage Guide](#3-usage-guide)
    * [3.1 Connecting to the Aleo Network](#31-connecting-to-the-aleo-network)
    * [3.2 Command Line Interface](#32-command-line-interface)
    * [3.3 Configuration File](#33-configuration-file)
* [4. JSON-RPC Interface](#4-json-rpc-interface)
* [5. Additional Information](#5-additional-information)
* [6. License](#6-license)

## 1. Overview

__snarkOS__ is a decentralized operating system for private applications. It forms the backbone of [Aleo](https://aleo.org/) and 
enables applications to verify and store state in a publicly verifiable manner.

## 2. Build Guide

### 2.1 Install Rust

We recommend installing Rust using [rustup](https://www.rustup.rs/). You can install `rustup` as follows:

- macOS or Linux:
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- Windows (64-bit):  
  
  Download the [Windows 64-bit executable](https://win.rustup.rs/x86_64) and follow the on-screen instructions.

- Windows (32-bit):  
  
  Download the [Windows 32-bit executable](https://win.rustup.rs/i686) and follow the on-screen instructions.

### 2.2a Build from Crates.io

We recommend installing snarkOS 1.1.4 way. In your terminal, run:

```bash
cargo install snarkos
```

Now to use snarkOS, in your terminal, run:
```bash
snarkos
```
 
### 2.2b Build from Source Code

Alternatively, you can install snarkOS 1.1.4 building from the source code as follows:

```bash
# Download the source code
git clone https://github.com/AleoHQ/snarkOS
cd snarkOS

# Build in release mode
$ cargo build --release
```

This will generate an executable under the `./target/release` directory. To run snarkOS, run the following command:
```bash
./target/release/snarkos
```

### 2.2c Build with Docker

#### Docker build
```bash
docker build -t snarkos:latest .
``` 
or 
```bash
docker-compose build
```

#### Docker run
``` bash
docker run -d -p 4131:4131 --name snarkos snarkos 
```
or
```bash
docker-compose up
```

## 3. Usage Guide

### 3.1 Connecting to the Aleo network

To start a client node, run:
```
snarkos
```

To start a mining node, run:
```
snarkos --is-miner
```

To run a node with custom settings, refer to the full list of options and flags available 
in the CLI or simply modify the snarkOS 1.1.4 file.

### 3.2 Command Line Interface

Full list of CLI flags and options can be viewed with `snarkos --help`:

```
snarkOS 1.1.4
Run an Aleo node (include -h for more options)

USAGE:
    snarkos [FLAGS] [OPTIONS]

FLAGS:
    -h, --help           Prints help information
        --is-bootnode    Run the node as a bootnode (IP is hard coded in the protocol)
        --is-miner       Start mining blocks from this node
        --no-jsonrpc     Run the node without running the json rpc server

OPTIONS:
        --connect <ip>                           Specify one or more node ip addresses to connect to on startup
    -i, --ip <ip>                                Specify the ip of your node
        --max-peers <max-peers>                  Specify the maximum number of peers the node can connect to
        --mempool-interval <mempool-interval>    Specify the frequency in seconds the node should fetch a sync node's mempool
        --min-peers <min-peers>                  Specify the minimum number of peers the node should connect to
        --miner-address <miner-address>          Specify the address that will receive miner rewards
        --network <network-id>                   Specify the network id (default = 1) of the node
    -d, --path <path>                            Specify the node's storage path
    -p, --port <port>                            Specify the port the node is run on
        --rpc-password <rpc-password>            Specify a password for rpc authentication
        --rpc-port <rpc-port>                    Specify the port the json rpc server is run on
        --rpc-username <rpc-username>            Specify a username for rpc authentication
        --verbose <verbose>                      Specify the verbosity (default = 1) of the node [possible values: 0, 1, 2]
```

#### Examples

##### Guard RPC endpoints
```
snarkos --rpc-username <Username> --rpc-password <Password>
```

##### Manually connect to a peer on the network
```
snarkos --connect "<IP ADDRESS>"
```

### 3.3 Configuration File

A `config.toml` file is generated in the `~/.snarkOS/` directory when the node is initialized for the time. 
Updating this `config.toml` file allows node operators to specify default settings for the node without 
having to specify additional information in the CLI.

## 4. JSON-RPC Interface

By default, snarkOS 1.1.4 a JSON-RPC server to allow external interfacing with the Aleo network. Documentation of the RPC endpoints can be found [here](rpc/README.md)

## 5. Additional Information

For additional information, please refer to the official [Aleo documentation page](https://developer.aleo.org/aleo/getting_started/overview/).

## 6. License

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](./LICENSE.md)
