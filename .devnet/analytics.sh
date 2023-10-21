#!/bin/bash

# Get the directory of the bash script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Navigate to the directory containing the JavaScript program
cd "$SCRIPT_DIR/analytics"

# Check if the 'node_modules' directory exists
if [ ! -d "node_modules" ]; then
  echo "Node.js dependencies not found. Running 'npm install'..."
  npm install
else
  echo "Node.js dependencies already installed."
fi

# Call the JavaScript program using Node.js
npm start
