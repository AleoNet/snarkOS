#!/bin/bash

# Determine the number of AWS EC2 instances by checking ~/.ssh/config
NODE_ID=0
while [ -n "$(grep "aws-n${NODE_ID}" ~/.ssh/config)" ]; do
    NODE_ID=$((NODE_ID + 1))
done

# Read the number of AWS EC2 instances to query from the user
read -p "Enter the number of AWS EC2 instances to query (default: $NODE_ID): " NUM_INSTANCES
NUM_INSTANCES="${NUM_INSTANCES:-$NODE_ID}"

echo "Using $NUM_INSTANCES AWS EC2 instances for querying."

# Read the network ID from user or use a default value of 1
read -p "Enter the network ID (mainnet = 0, testnet = 1, canary = 2) (default: 1): " NETWORK_ID
NETWORK_ID=${NETWORK_ID:-1}

echo "Using network ID $NETWORK_ID."

# Define a function to terminate the tmux session on a node
terminate_tmux_session() {
  local NODE_ID=$1

  # SSH into the node and send the "tmux kill-session" command to terminate the tmux session
  ssh -o StrictHostKeyChecking=no aws-n$NODE_ID << EOF
    # Command to terminate the tmux session
    sudo -i  # Switch to root user
    WORKSPACE=~/snarkOS
    cd \$WORKSPACE

    tmux kill-session -t snarkos-session
    snarkos clean --dev $NODE_ID --network $NETWORK_ID

    exit  # Exit root user
EOF

  # Check the exit status of the SSH command
  if [ $? -eq 0 ]; then
    echo "tmux session terminated successfully on aws-n$NODE_ID."
  else
    echo "Failed to terminate tmux session on aws-n$NODE_ID."
  fi
}

# Loop through aws-n nodes and terminate tmux sessions in parallel
for NODE_ID in $(seq 0 $(($NUM_INSTANCES - 1))); do
  terminate_tmux_session $NODE_ID &
done

# Wait for all background jobs to finish
wait
