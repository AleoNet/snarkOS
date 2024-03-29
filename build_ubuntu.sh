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


# Check if Rust is installed, if not, install Rust.

if command -v rustc &> /dev/null
then
    echo "Rust is already installed. Skipping installation."
else
    # Install Rust
    echo "Rust is not installed. Installing now..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME"/.cargo/env
    echo "Rust installation complete."
fi

# Install snarkOS
# cargo clean
cargo install --locked --path .

echo "=================================================="
echo " Attention - Please ensure ports 4133 and 3033"
echo "             are enabled on your local network."
echo ""
echo " Cloud Providers - Enable ports 4133 and 3033"
echo "                   in your network firewall"
echo ""
echo " Home Users - Enable port forwarding or NAT rules"
echo "              for 4133 and 3033 on your router."
echo "=================================================="
