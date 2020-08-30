#!/bin/bash
if [[ $(/usr/bin/id -u) -ne 0 ]]; then
    echo "Aborting: run as root user!"
    exit 1
fi

echo "================================================"
echo " Attention - Building snarkOS from source code."
echo "================================================"

# Install Ubuntu dependencies

apt-get update
apt-get install -y \
    build-essential \
    clang \
    gcc \
    libssl-dev \
    llvm \
    make \
    pkg-config \
    xz-utils

# Install Rust

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# Build snarkOS Release

cargo build --release

# Start snarkOS

echo "================================================"
echo " Attention - Please ensure ports 4131 and 3030"
echo "             are enabled on your local network."
echo "================================================"

./target/release/snarkos
