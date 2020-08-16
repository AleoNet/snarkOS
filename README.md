<h1 align="center">snarkOS</h1>

<p align="center">
    <a href="https://circleci.com/gh/AleoHQ/snarkOS"><img src="https://circleci.com/gh/AleoHQ/snarkOS.svg?style=svg&circle-token=6e9ad6d39d95350544f352d34e0e5c62ef54db26"></a>
    <a href="https://codecov.io/gh/AleoHQ/snarkOS"><img src="https://codecov.io/gh/AleoHQ/snarkOS/branch/master/graph/badge.svg?token=cck8tS9HpO"/></a>
</p>

__snarkOS__ is a decentralized operating system for private applications.

## <a name='TableofContents'></a>Table of Contents

* [1. Overview](#1-overview)
* [2. Build Guide](#2-build-guide)
    * [2.1 Install Rust](#21-install-rust)
    * [2.2a Build from Crates.io](#22b-build-from-cratesio)
    * [2.2b Build from Source Code](#22c-build-from-source-code)
* [3. Usage Guide](#3-usage-guide)
* [4. License](#4-license)

## 1. Overview

\[WIP\]

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

We recommend installing snarkOS this way. In your terminal, run:

```bash
cargo install snarkos
```

Now to use snarkOS, in your terminal, run:
```bash
snarkos
```
 
### 2.2b Build from Source Code

Alternatively, you can install snarkOS by building from the source code as follows:

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

### 2.3b Run by Docker

#### Docker build
docker build -t snarkos:latest .
(or docker-compose build)

#### Docker run
docker run -d -p 4130:4130 --name snarkos snarkos
(or docker-compose up)

## 3. Usage Guide

To start a client node, run:
```
snarkos
```

To start a mining node, run:
```
snarkos --is-miner
```

#### How to guard RPC endpoints
```
./target/release/snarkOS --rpc-username <Username> --rpc-password <Password>
```

#### How to manually connect to a peer on the network
```
./target/release/snarkOS --connect "<IP ADDRESS>"
```

### Interfacing with a running node

By default, snarkOS runs a JSON-RPC server to allow external interfacing with the Aleo network. Additional information can be found [here](aleo/documentation/autogen/testnet/rpc/rpc_server/00_configurations.md)


## 4. License

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](./LICENSE.md)