#!/bin/bash

# USAGE examples: 
  # CLI with env vars: MINER_ADDRESS=aleoABCD...   ./run-operator.sh


# if env var MINER_ADDRESS is not set, prompt for it
if [ -z "${MINER_ADDRESS}" ]
then
  read -r -p "Enter your miner address for your operator: "
  MINER_ADDRESS=$REPLY
fi

if [ "${MINER_ADDRESS}" == "" ]
then
  MINER_ADDRESS="aleo1d5hg2z3ma00382pngntdp68e74zv54jdxy249qhaujhks9c72yrs33ddah"
fi

COMMAND="cargo run --release -- --operator ${MINER_ADDRESS} --trial --verbosity 2"

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

echo "Running miner node..."
$COMMAND &

while :
do
  echo "Checking for updates..."
  git stash
  rm Cargo.lock
  STATUS=$(git pull)

  if [ "$STATUS" != "Already up to date." ]; then
    echo "Updated code found, rebuilding and relaunching miner"
    cargo clean
    kill -INT $!; sleep 2; $COMMAND &
  fi

  sleep 1800;
done
