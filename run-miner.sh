#!/bin/bash

COMMAND='cargo run --release -- --miner aleo1d5hg2z3ma00382pngntdp68e74zv54jdxy249qhaujhks9c72yrs33ddah --trial'

function exit_node()
{
    echo "Exiting..."
    kill $!
    exit
}

trap exit_node SIGINT

echo "Running miner node..."

while :
do
  echo "Checking for updates..."
  git pull

  echo "Running the node..."
  $COMMAND & sleep 1800; kill $!

  sleep 2;
done
