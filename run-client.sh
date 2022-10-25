#!/bin/bash

# USAGE examples: 
  # CLI :  ./run-client.sh

COMMAND='cargo run --release -- start --nodisplay'

for word in $*;
do
  COMMAND="${COMMAND} ${word}"
done

function exit_node()
{
    echo "Exiting..."
    kill $!
    exit
}

trap exit_node SIGINT

echo "Running an Aleo client node..."
$COMMAND &

while :
do
  echo "Checking for updates..."
  git stash
  rm Cargo.lock
  STATUS=$(git pull)
  
  if [ "$STATUS" != "Already up to date." ]; then
    echo "Updated code found, rebuilding and relaunching client"
    cargo clean
    kill -INT $!; sleep 2; $COMMAND &
  fi
  
  sleep 1800

done
