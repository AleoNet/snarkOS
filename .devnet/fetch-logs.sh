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

# Define the directory where logs will be saved
log_directory="$HOME/snarkos_logs"

# Create the log directory if it doesn't already exist
mkdir -p "$log_directory"

# Loop from 0 to 49
for i in $(seq 0 $(($NUM_INSTANCES - 1))); do
    echo "Connecting to aws-n$i..."
    # Use sftp to connect, execute commands, and exit
    sftp aws-n$i << EOF
cd /tmp
get snarkos.log "$log_directory/snarkos-$i.log"
EOF
    echo "Downloaded snarkos.log from aws-n$i as snarkos-$i.log into $log_directory"
done

echo "All files have been downloaded to $log_directory."
