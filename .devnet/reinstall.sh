#!/bin/bash

# Prompt the user for the branch to install (default is "mainnet")
read -p "Enter the branch to install (default: mainnet): " BRANCH
BRANCH=${BRANCH:-mainnet}

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
#      git clone https://github.com/AleoNet/snarkOS.git \$WORKSPACE
      cd \$WORKSPACE
      git pull # If we are switching branches, this will find the new branch
      git checkout $BRANCH  # Checkout the specified branch
      git pull origin $BRANCH
    else
      # The workspace directory doesn't exist, clone the repository
      git clone https://github.com/AleoNet/snarkOS.git \$WORKSPACE
      cd \$WORKSPACE
      git checkout $BRANCH  # Checkout the specified branch
    fi

    cargo install --path .
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
for NODE_ID in $(seq 0 $(($NUM_INSTANCES - 1))); do
  run_installation $NODE_ID $BRANCH &
done

# Wait for all background jobs to finish
wait
