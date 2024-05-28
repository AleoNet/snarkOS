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

# Read the verbosity level from the user (default: 1)
read -p "Enter the verbosity level (default: 1): " VERBOSITY
VERBOSITY="${VERBOSITY:-1}"

echo "Using verbosity level $VERBOSITY."

# Get the IP address of NODE 0 from the SSH config for aws-n0
NODE_0_IP=$(awk '/Host aws-n0/{f=1} f&&/HostName/{print $2; exit}' ~/.ssh/config)

# Define a function to start snarkOS in a tmux session on a node
start_snarkos_in_tmux() {
  local NODE_ID=$1
  local NODE_IP=$2

  # SSH into the node and start snarkOS in a new tmux session
  ssh -o StrictHostKeyChecking=no aws-n$NODE_ID << EOF
    # Commands to run on the remote instance
    sudo -i  # Switch to root user
    WORKSPACE=~/snarkOS
    cd \$WORKSPACE

    # Start snarkOS within a new tmux session named "snarkos-session"
    tmux new-session -d -s snarkos-session

    # Send the snarkOS start command to the tmux session with the NODE_ID
    tmux send-keys -t "snarkos-session" "snarkos start --client --nocdn --nodisplay --rest 0.0.0.0:3030 --node 0.0.0.0:4130 --verbosity 4 --network $NETWORK_ID --metrics --logfile "/tmp/snarkos-syncing-range-3.log" --peers 167.71.249.65:4130,157.245.218.195:4130,167.71.249.55:4130" C-m

    exit  # Exit root user
EOF

  # Check the exit status of the SSH command
  if [ $? -eq 0 ]; then
    echo "snarkOS started successfully in a tmux session on aws-n$NODE_ID."
  else
    echo "Failed to start snarkOS in a tmux session on aws-n$NODE_ID."
  fi
}

# Loop through aws-n nodes and start snarkOS in tmux sessions in parallel
for NODE_ID in $(seq 0 $(($NUM_INSTANCES - 1))); do
  start_snarkos_in_tmux $NODE_ID "$NODE_0_IP" &
done

# Wait for all background jobs to finish
wait
