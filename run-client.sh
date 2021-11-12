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
  $COMMAND & sleep 3600; kill $!

  sleep 2;
done
