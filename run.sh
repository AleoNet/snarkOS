#!/bin/bash

COMMAND='cargo run --release -- --miner aleo1d5hg2z3ma00382pngntdp68e74zv54jdxy249qhaujhks9c72yrs33ddah  --trial'

function exit_node()
{
    echo "Exiting..."
    exit 0
}

trap exit_node SIGINT

while :
do
  echo "Checking for updates..."
  git pull

  echo "Running the node..."
  $COMMAND & sleep 3600; kill $!

  sleep 2;
done
