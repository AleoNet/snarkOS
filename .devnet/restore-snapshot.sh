#!/bin/bash

# Determine the number of nodes by checking ~/.ssh/config
NODE_ID=0
while [ -n "$(grep "aws-n${NODE_ID}" ~/.ssh/config)" ]; do
    NODE_ID=$((NODE_ID + 1))
done

# Read the number of nodes to query from the user
read -p "Enter the number of nodes to query (default: $NODE_ID): " NUM_NODES
NUM_NODES="${NUM_NODES:-$NODE_ID}"

echo "Using $NUM_NODES nodes for querying."

# Ask for the URL and the file name from the user
read -p "Enter the URL to download: " DOWNLOAD_URL
read -p "Enter the file name to extract: " FILE_NAME

# Define a function to run a script on a node
run_in_ssh() {
  local NODE_ID=$1

  # SSH into the node and run a script
  ssh -o StrictHostKeyChecking=no aws-n$NODE_ID << EOF
    # Commands to run
    sudo -i  # Switch to root user
    WORKSPACE=~/snarkOS

    # Download and extract the snapshot
    wget $DOWNLOAD_URL
    tar -xvf $FILE_NAME

    # Remove the old ledger
    snarkos clean --dev $NODE_ID

    # Move the new ledger into place
    cp -R ledger-3 \$WORKSPACE/.ledger-3-$NODE_ID

    exit  # Exit root user
EOF

  # Check the exit status of the SSH command
  if [ $? -eq 0 ]; then
    echo "Script ran successfully on aws-n$NODE_ID."
  else
    echo "Failed to run the script on aws-n$NODE_ID."
  fi
}

# Loop through the nodes and run the script in parallel
for NODE_ID in $(seq 0 $NUM_NODES); do
  run_in_ssh $NODE_ID &
done

# Wait for all background jobs to finish
wait
