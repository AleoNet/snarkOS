#!/bin/bash

# Read the total number of validators from the user or use a default value of 4
read -p "Enter the total number of validators (default: 4): " total_validators
total_validators=${total_validators:-4}

# Ask the user if they want to run 'cargo install --path .' or use a pre-installed binary
read -p "Do you want to run 'cargo install --path .' to build the binary? (y/n, default: y): " build_binary
build_binary=${build_binary:-y}

# Ask the user whether to clear the existing ledger logs
read -p "Do you want to clear the existing ledger logs? (y/n, default: n): " clear_logs
clear_logs=${clear_logs:-n}

if [[ $build_binary == "y" ]]; then
  # Build the binary using 'cargo install --path .'
  cargo install --path . || exit 1
fi

# Clear the ledger logs for each validator if the user chooses to clear logs
if [[ $clear_logs == "y" ]]; then
  # Create an array to store background processes
  clean_processes=()

  for ((validator_index = 0; validator_index < total_validators; validator_index++)); do
    # Run 'snarkos clean' for each validator in the background
    snarkos clean --dev $validator_index &

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

# Generate validator indices from 0 to (total_validators - 1)
validator_indices=($(seq 0 $((total_validators - 1))))

# Loop through the list of validator indices and create a new window for each
for validator_index in "${validator_indices[@]}"; do
  # Generate a unique and incrementing log file name based on the validator index
  log_file="$log_dir/validator-$validator_index.log"

  # Create a new window with a unique name
  tmux new-window -t "devnet:$validator_index" -n "window$validator_index"

  # Send the command to start the validator to the new window and capture output to the log file
  tmux send-keys -t "devnet:window$validator_index" "snarkos start --nodisplay --dev $validator_index --dev-num-validators $total_validators --validator --logfile $log_file" C-m
done

# Attach to the tmux session to view and interact with the windows
tmux attach-session -t "devnet"
