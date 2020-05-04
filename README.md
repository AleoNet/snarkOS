<h1 align="center">snarkOS</h1>

<p align="center">
    <a href="https://travis-ci.com/AleoHQ/snarkOS"><img src="https://travis-ci.com/AleoHQ/snarkOS.svg?token=Xy7ht9JdPvr4xSgbPruF&branch=master"></a>
    <a href="https://codecov.io/gh/AleoHQ/snarkOS"><img src="https://codecov.io/gh/AleoHQ/snarkOS/branch/master/graph/badge.svg?token=cck8tS9HpO"/></a>
</p>

__snarkOS__ is a decentralized operating system for confidential programs.

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

## 3. Usage Guide

\[WIP\]

## 4. License

[LICENSE](./LICENSE.md)