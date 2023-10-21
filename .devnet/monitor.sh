#!/bin/bash

# Read the number of AWS EC2 instances to query from the user
read -p "Enter the number of AWS EC2 instances to query (default: 16): " NUM_INSTANCES
NUM_INSTANCES="${NUM_INSTANCES:-16}"

# Create a new local tmux session named "devnet-aws"
tmux new-session -d -s "devnet-aws" -n "window0"

# Generate validator indices from 0 to (NUM_INSTANCES - 1)
validator_indices=($(seq 0 $((NUM_INSTANCES - 1))))

# Loop through the list of validator indices and create a new window for each
for validator_index in "${validator_indices[@]}"; do
  # Create a new window with a unique name
  tmux new-window -t "devnet-aws:$validator_index" -n "window$validator_index"

  # Define the SSH command to run on the remote instance
  ssh_command="sudo -i tmux attach-session -t 'snarkos-session'"

  # Send the SSH command to the new window
  tmux send-keys -t "devnet-aws:window$validator_index" "ssh -t -o StrictHostKeyChecking=no aws-n$validator_index \"$ssh_command\"" C-m
done

# Attach to the tmux session to view and interact with the windows
tmux attach-session -t "devnet-aws"


## Read the node ID to query from the user
#read -p "Enter the node ID (default: 0): " NODE_ID
#NODE_ID="${NODE_ID:-0}"
#
## SSH into the node and attach to the snarkOS tmux session
##ssh -o StrictHostKeyChecking=no aws-n$NODE_ID << EOF
##  # Commands to run on the remote instance
##  sudo -i  # Switch to root user
##  tmux attach-session -t "snarkos-session"
##EOF
#ssh -t -o StrictHostKeyChecking=no aws-n$NODE_ID "sudo -i tmux attach-session -t 'snarkos-session'"
