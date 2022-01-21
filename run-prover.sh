#!/bin/bash

# USAGE examples: 
  # CLI with env vars: MINER_ADDRESS=aleoABCD...  OPERATOR_IP_ADDRESS=a.b.c.d ./run-prover.sh
  # CLI with prompts for vars:  ./run-prover.sh


# if env var MINER_ADDRESS is not set, prompt for it
if [ -z "${MINER_ADDRESS}" ]
then
  read -r -p "Enter your miner address: "
  MINER_ADDRESS=$REPLY
fi

if [ "${MINER_ADDRESS}" == "" ]
then
  MINER_ADDRESS="aleo1d5hg2z3ma00382pngntdp68e74zv54jdxy249qhaujhks9c72yrs33ddah"
fi

# if env var OPERATOR_IP_ADDRESS is not set, prompt for it
if [ -z "${OPERATOR_IP_ADDRESS}" ]
then
  read -r -p "Enter your Operator Servers IP address: "
  OPERATOR_IP_ADDRESS=$REPLY
fi

if [ "${OPERATOR_IP_ADDRESS}" == "" ]
then
  echo "IP Address of Operator server is required to run a prover"
  exit 1
fi

# if env var OPERATOR_IP_ADDRESS is not set, use default port of 4132
if [ -z "${OPERATOR_IP_PORT}" ]
then
  OPERATOR_IP_PORT=4132
fi

COMMAND="cargo run --release -- --prover ${MINER_ADDRESS} --pool ${OPERATOR_IP_ADDRESS}:${OPERATOR_IP_PORT} --trial --verbosity 2"

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

echo "Running prover node..."
$COMMAND &


while :
do
  echo "Checking for updates..."
  git stash
  rm Cargo.lock
  STATUS=$(git pull)

  if [ "${STATUS}" != "Already up to date." ]; then
    echo "Updated code found, rebuilding and relaunching miner"
    cargo clean
    kill -INT $!; sleep 2; $COMMAND &
  fi

  sleep 1800;
done
