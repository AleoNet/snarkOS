#!/bin/bash
# USAGE examples: 
  # CLI with env vars: PROVER_ADDRESS=aleo1zkp...  ./run-prover.sh
  # CLI with prompts for vars:  ./run-prover.sh

# If the env var PROVER_ADDRESS is not set, prompt for it
if [ -z "${PROVER_ADDRESS}" ]
then
  read -r -p "Enter the Aleo Prover account address: "
  PROVER_ADDRESS=$REPLY
fi

if [ "${PROVER_ADDRESS}" == "" ]
then
  PROVER_ADDRESS="aleo1wvgwnqvy46qq0zemj0k6sfp3zv0mp77rw97khvwuhac05yuwscxqmfyhwf"
fi

COMMAND="cargo run --release -- start --nodisplay --prover ${PROVER_ADDRESS}"

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

echo "Running an Aleo Prover node..."
$COMMAND &

while :
do
  echo "Checking for updates..."
  git stash
  rm Cargo.lock
  STATUS=$(git pull)

  if [ "$STATUS" != "Already up to date." ]; then
    echo "Updated code found, rebuilding and relaunching prover"
    cargo clean
    kill -INT $!; sleep 2; $COMMAND &
  fi

  sleep 1800;
done
