#!/bin/bash

COMMAND='cargo run --release -- --trial --verbosity 2'

function exit_node()
{
    echo "Exiting..."
    kill -INT $!
    exit
}

trap exit_node SIGINT

echo "Running client node..."

while :
do
  echo "Checking for updates..."
  STATUS=$(git pull)

  echo "Running the node..."
  
  if [ "$STATUS" != "Already up to date." ]; then
    cargo clean
  fi

  $COMMAND & sleep 1800; kill -INT $!

  sleep 2;
done
