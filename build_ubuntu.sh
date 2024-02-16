#!/bin/bash
echo "================================================"
echo " Attention - Building snarkOS from source code."
echo " This will request root permissions with sudo."
echo "================================================"

# Install Ubuntu dependencies
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    curl \
    clang \
    gcc \
    libssl-dev \
    llvm \
    make \
    pkg-config \
    tmux \
    xz-utils \
    ufw

# Check if Ubuntu dependencies installation was successful
if [ $? -ne 0 ]; then
    echo "Error: Failed to install Ubuntu dependencies. Please check your internet connection and try again."
    exit 1
fi

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# Check if Rust installation was successful
if [ $? -ne 0 ]; then
    echo "Error: Failed to install Rust. Please check your internet connection and try again."
    exit 1
fi

# Install snarkOS
cargo install --locked --path .

# Check if snarkOS installation was successful
if [ $? -ne 0 ]; then
    echo "Error: Failed to install snarkOS. Please check for any error messages above."
    exit 1
fi

echo "=================================================="
echo " Attention - Please ensure ports 4130 and 3030"
echo "             are enabled on your local network."
echo ""
echo " Cloud Providers - Enable ports 4130 and 3030"
echo "                   in your network firewall"
echo ""
echo " Home Users - Enable port forwarding or NAT rules"
echo "              for 4130 and 3030 on your router."
echo "=================================================="
