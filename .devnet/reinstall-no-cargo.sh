#!/bin/bash

# Prompt the user for the branch to install (default is "testnet3")
read -p "Enter the branch to install (default: testnet3): " BRANCH
BRANCH=${BRANCH:-testnet3}

# Determine the number of AWS EC2 instances by checking ~/.ssh/config
NODE_ID=0
while [ -n "$(grep "aws-n${NODE_ID}" ~/.ssh/config)" ]; do
    NODE_ID=$((NODE_ID + 1))
done

# Read the number of AWS EC2 instances to query from the user
read -p "Enter the number of AWS EC2 instances to query (default: $NODE_ID): " NUM_INSTANCES
NUM_INSTANCES="${NUM_INSTANCES:-$NODE_ID}"

echo "Using $NUM_INSTANCES AWS EC2 instances for querying."

# Define a function to run the installation on a node
run_installation() {
  local NODE_ID=$1
  local BRANCH=$2
  # SSH into the node
  ssh -o StrictHostKeyChecking=no aws-n$NODE_ID << EOF
    # Commands to run on the remote instance
    sudo -i  # Switch to root user
    WORKSPACE=~/snarkOS

    if [ -d "\$WORKSPACE" ]; then
      # The workspace directory exists, update the existing repository
#      rm -rf \$WORKSPACE
#      git clone https://github.com/AleoHQ/snarkOS.git \$WORKSPACE
      cd \$WORKSPACE
      git pull # If we are switching branches, this will find the new branch
      git checkout $BRANCH  # Checkout the specified branch
      git pull origin $BRANCH
    else
      # The workspace directory doesn't exist, clone the repository
      git clone https://github.com/AleoHQ/snarkOS.git \$WORKSPACE
      cd \$WORKSPACE
      git checkout $BRANCH  # Checkout the specified branch
    fi

    cargo build --release

    # Assuming the binary is named snarkOS and is located in the target/release directory
    BINARY_PATH="/root/snarkOS/target/release/snarkos"  # Use the absolute path
    INSTALL_PATH="/usr/local/bin/snarkOS"

    # Check if the binary exists
    if [ -f "\$BINARY_PATH" ]; then
        echo "Binary found, proceeding with installation."
        
        # Optionally backup the existing binary
        if [ -f "\$INSTALL_PATH" ]; then
            echo "Existing binary found, creating a backup."
            mv "\$INSTALL_PATH" "\${INSTALL_PATH}.bak"
        fi
        
        # Copy the new binary to the desired location
        cp "\$BINARY_PATH" "\$INSTALL_PATH"
        
        # Give execution rights to the binary
        chmod +x "\$INSTALL_PATH"
        
        echo "snarkOS installed successfully at \$INSTALL_PATH"
        
        # Clear the hash table to make the shell aware of the new binary's path
        hash -r
        echo "Shell hash table cleared. 'snarkOS' command should now be recognized immediately."
    else
        echo "Error: Binary not found at \$BINARY_PATH. Installation aborted."
    fi

    exit  # Exit root user
EOF


  # Check the exit status of the SSH command
  if [ $? -eq 0 ]; then
    echo "Installation on aws-n$NODE_ID completed successfully."
  else
    echo "Installation on aws-n$NODE_ID failed."
  fi
}

# Loop through aws-n nodes and run installations in parallel
for NODE_ID in $(seq 0 $NUM_INSTANCES); do
  run_installation $NODE_ID $BRANCH &
done

# Wait for all background jobs to finish
wait
