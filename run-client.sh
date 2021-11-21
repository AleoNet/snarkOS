#!/bin/bash

COMMAND='cargo run --release -- --trial'

function exit_node()
{
    echo "Exiting..."
    kill $!
    exit
}

trap exit_node SIGINT

echo "Running client node..."

while :
do
  echo "Checking for updates..."
  git pull

  echo "Running the node..."
  cargo clean
  $COMMAND & sleep 1800; kill $!

  sleep 2;
done
