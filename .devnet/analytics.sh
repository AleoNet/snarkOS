#!/bin/bash

# Get the directory of the bash script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Navigate to the directory containing the JavaScript program
cd "$SCRIPT_DIR/.analytics"

# Check if the 'node_modules' directory exists
if [ ! -d "node_modules" ]; then
  echo "Node.js dependencies not found. Running 'npm install'..."
  npm install
else
  echo "Node.js dependencies already installed."
fi

# Prompt the user to specify a network ID
while true; do
  echo "Please specify a network ID (0 for mainnet, 1 for testnet, 2 for canary):"
  read networkID
  if [[ $networkID == 0 || $networkID == 1 || $networkID == 2 ]]; then
    break
  else
    echo "Invalid network ID. Please enter 0 or 1."
  fi
done

# Prompt the user to select a metric type
PS3="Select a metric type: "
options=("Average Block Time" "Rounds in Blocks" "Check Block Hash" "Quit")
select opt in "${options[@]}"
do
  case $opt in
    "Average Block Time")
      echo ""
      node analytics.js --metric-type averageBlockTime --network-id $networkID
      break
      ;;
    "Rounds in Blocks")
      echo ""
      node analytics.js --metric-type roundsInBlocks --network-id $networkID
      break
      ;;
    "Check Block Hash")
      echo "You selected 'Check Block Hash'. Please enter the block height:"
      read blockHeight
      echo ""
      # Validate input is an integer
      if ! [[ "$blockHeight" =~ ^[0-9]+$ ]]; then
        echo "Error: Block height must be a positive integer."
        exit 1
      fi
      node analytics.js --metric-type checkBlockHash --block-height "$blockHeight" --network-id $networkID
      break
      ;;
    "Quit")
      echo "Quitting..."
      break
      ;;
    *) echo "Invalid option";;
  esac
done
