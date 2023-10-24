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

# Prompt the user to select a metric type
PS3="Select a metric type: "
options=("Average Block Time" "Rounds in Blocks" "Quit")
select opt in "${options[@]}"
do
  case $opt in
    "Average Block Time")
      npm run averageBlockTime
      break
      ;;
    "Rounds in Blocks")
      npm run roundsInBlocks
      break
      ;;
    "Quit")
      echo "Quitting..."
      break
      ;;
    *) echo "Invalid option";;
  esac
done
