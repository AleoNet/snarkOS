#!/bin/bash

COMMAND='cargo run --release -- --trial --verbosity 2'

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

echo "Running client node..."

while :
do
  echo "Checking for updates..."
  git stash
  STATUS=$(git pull)

  echo "Running the node..."
  
  if [ "$STATUS" != "Already up to date." ]; then
    cargo clean
  fi

  $COMMAND & sleep 1800; kill -INT $!

  sleep 2;
done
