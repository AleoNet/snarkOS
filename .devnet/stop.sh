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

# Define a function to terminate the tmux session on a node
terminate_tmux_session() {
  local NODE_ID=$1

  # SSH into the node and send the "tmux kill-session" command to terminate the tmux session
  ssh -o StrictHostKeyChecking=no aws-n$NODE_ID << EOF
    # Command to terminate the tmux session
    sudo -i  # Switch to root user
    tmux kill-session -t snarkos-session

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
