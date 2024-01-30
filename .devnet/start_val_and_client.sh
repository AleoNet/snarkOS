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

# Calculate the number of validator and client nodes
NUM_VALIDATORS=$((NUM_INSTANCES - 2))
NUM_CLIENTS=2

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
  local NODE_TYPE=$3  # Either "validator" or "client"

  # SSH into the node and start snarkOS in a new tmux session
  ssh -o StrictHostKeyChecking=no aws-n$NODE_ID << EOF
    sudo -i
    WORKSPACE=~/snarkOS
    cd \$WORKSPACE
    tmux new-session -d -s snarkos-session

    if [ "$NODE_TYPE" = "validator" ]; then
      tmux send-keys -t "snarkos-session" "snarkos start --nodisplay --bft 0.0.0.0:5000 --rest 0.0.0.0:3033 --peers $NODE_IP:4133 --validators $NODE_IP:5000 --verbosity $VERBOSITY --dev $NODE_ID --dev-num-validators $NUM_VALIDATORS --validator --metrics" C-m
    else
      tmux send-keys -t "snarkos-session" "snarkos start --nodisplay --peers $NODE_IP:4133 --client --dev-num-validators $NUM_VALIDATORS" C-m
    fi

    exit
EOF

  if [ $? -eq 0 ]; then
    echo "snarkOS started successfully in a tmux session on aws-n$NODE_ID."
  else
    echo "Failed to start snarkOS in a tmux session on aws-n$NODE_ID."
  fi
}

# Loop through nodes and start them as either validators or clients
for NODE_ID in $(seq 0 $((NUM_INSTANCES - 1))); do
  if [ $NODE_ID -lt $NUM_VALIDATORS ]; then
    start_snarkos_in_tmux $NODE_ID "$NODE_0_IP" "validator" &
  else
    start_snarkos_in_tmux $NODE_ID "$NODE_0_IP" "client" &
  fi
done

# Wait for all background jobs to finish
wait
