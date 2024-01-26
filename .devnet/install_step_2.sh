#!/bin/bash

# Prompt the user for the branch to install (default is "testnet3")
read -p "Enter the branch to install (default: kp/fix/stress_test_8): " BRANCH
BRANCH=${BRANCH:-kp/fix/stress_test_8}

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
      cd \$WORKSPACE
      git pull origin $BRANCH
    else
      # The workspace directory doesn't exist, clone the repository
      git clone https://github.com/AleoHQ/snarkOS.git \$WORKSPACE
      cd \$WORKSPACE
      git checkout $BRANCH  # Checkout the specified branch
    fi

    echo "before changing dependencies"
    sudo sed -i 's/aleo_std::aleo_ledger_dir(network_id, dev/aleo_std::aleo_ledger_dir(network_id, dev.into()/g' /root/.cargo/registry/src/index.crates.io-6f17d22bba15001f/aleo-std-storage-0.1.7/src/lib.rs
    sudo sed -i 's/aleo_std::aleo_ledger_dir(network_id, dev/aleo_std::aleo_ledger_dir(network_id, dev.into()/g' /root/.cargo/git/checkouts/snarkvm-f1160780ffe17de8/9656a7e/ledger/store/src/helpers/rocksdb/internal/mod.rs
    echo "after changing dependencies"

    sudo ./build_ubuntu.sh
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
