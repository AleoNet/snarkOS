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
  git stash
  STATUS=$(git pull)

  if [ "$STATUS" != "Already up to date." ]; then
    cargo clean
  fi

  $COMMAND & sleep 1800; kill -INT $!

  sleep 2;
done
