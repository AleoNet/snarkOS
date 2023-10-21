#!/bin/bash

# Prompt the user for the branch to install (default is "testnet3")
read -p "Enter the branch to install (default: testnet3): " BRANCH
BRANCH=${BRANCH:-testnet3}

# Read the number of AWS EC2 instances to query from the user
read -p "Enter the number of AWS EC2 instances to query (default: 16): " NUM_INSTANCES
NUM_INSTANCES="${NUM_INSTANCES:-16}"

# Define a function to run the installation on a node
run_installation() {
  local NODE_ID=$1
  local BRANCH=$2
  # SSH into the node
  ssh -o StrictHostKeyChecking=no aws-n$NODE_ID << EOF
    # Commands to run on the remote instance
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

    ./.devnet/.ssh-install.sh

    # Exit the SSH session
    exit
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
