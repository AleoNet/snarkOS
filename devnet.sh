#!/bin/bash

# Read the total number of validators from the user or use a default value of 4
read -p "Enter the total number of validators (default: 4): " total_validators
total_validators=${total_validators:-4}

# Read the total number of clients from the user or use a default value of 2
read -p "Enter the total number of clients (default: 2): " total_clients
total_clients=${total_clients:-2}

# Read the network ID from user or use a default value of 1
read -p "Enter the network ID (mainnet = 0, testnet = 1, canary = 2) (default: 1): " network_id
network_id=${network_id:-1}

# Ask the user if they want to run 'cargo install --locked --path .' or use a pre-installed binary
read -p "Do you want to run 'cargo install --locked --path .' to build the binary? (y/n, default: y): " build_binary
build_binary=${build_binary:-y}

# Ask the user whether to clear the existing ledger history
read -p "Do you want to clear the existing ledger history? (y/n, default: n): " clear_ledger
clear_ledger=${clear_ledger:-n}

if [[ $build_binary == "y" ]]; then
  # Build the binary using 'cargo install --path .'
  cargo install --locked --path . || exit 1
fi

# Clear the ledger logs for each validator if the user chooses to clear ledger
if [[ $clear_ledger == "y" ]]; then
  # Create an array to store background processes
  clean_processes=()

  for ((index = 0; index < $((total_validators + total_clients)); index++)); do
    # Run 'snarkos clean' for each node in the background
    snarkos clean --network $network_id --dev $index &

    # Store the process ID of the background task
    clean_processes+=($!)
  done

  # Wait for all 'snarkos clean' processes to finish
  for process_id in "${clean_processes[@]}"; do
    wait "$process_id"
  done
fi

# Create a timestamp-based directory for log files
log_dir=".logs-$(date +"%Y%m%d%H%M%S")"
mkdir -p "$log_dir"

# Create a new tmux session named "devnet"
tmux new-session -d -s "devnet" -n "window0"

# Get the tmux's base-index for windows
# we have to create all windows with index offset by this much
index_offset="$(tmux show-option -gv base-index)"
if [ -z "$index_offset" ]; then
  index_offset=0
fi

# Generate validator indices from 0 to (total_validators - 1)
validator_indices=($(seq 0 $((total_validators - 1))))

# Loop through the list of validator indices and create a new window for each
for validator_index in "${validator_indices[@]}"; do
  # Generate a unique and incrementing log file name based on the validator index
  log_file="$log_dir/validator-$validator_index.log"

  # Send the command to start the validator to the new window and capture output to the log file
  if [ "$validator_index" -eq 0 ]; then
    tmux send-keys -t "devnet:window$validator_index" "snarkos start --nodisplay --network $network_id --dev $validator_index --allow-external-peers --dev-num-validators $total_validators --validator --logfile $log_file --metrics" C-m
  else
    # Create a new window with a unique name
    window_index=$((validator_index + index_offset))
    tmux new-window -t "devnet:$window_index" -n "window$validator_index"
    tmux send-keys -t "devnet:window$validator_index" "snarkos start --nodisplay --network $network_id --dev $validator_index --allow-external-peers --dev-num-validators $total_validators --validator --logfile $log_file" C-m
  fi
done

# Generate client indices from 0 to (total_clients - 1)
client_indices=($(seq 0 $((total_clients - 1))))

# Loop through the list of client indices and create a new window for each
for client_index in "${client_indices[@]}"; do
  # Generate a unique and incrementing log file name based on the client index
  log_file="$log_dir/client-$client_index.log"

  window_index=$((client_index + total_validators + index_offset))

  # Create a new window with a unique name
  tmux new-window -t "devnet:$window_index" -n "window-$window_index"

  # Send the command to start the validator to the new window and capture output to the log file
  tmux send-keys -t "devnet:window-$window_index" "snarkos start --nodisplay --network $network_id --dev $window_index --dev-num-validators $total_validators --client --logfile $log_file" C-m
done

# Attach to the tmux session to view and interact with the windows
tmux attach-session -t "devnet"
