#!/bin/bash

COMMAND='cargo run --release -- --sync --trial'

function exit_node()
{
    echo "Exiting..."
    kill $!
    exit
}

trap exit_node SIGINT

echo "Do not run a sync node, it does nothing..."

while :
do
  echo "Checking for updates..."
  git pull

  cargo clean
  $COMMAND & sleep 1800; kill $!

  sleep 2;
done
