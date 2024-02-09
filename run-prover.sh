#!/bin/bash
# USAGE examples: 
  # CLI with env vars: PROVER_PRIVATE_KEY=APrivateKey1...  ./run-prover.sh
  # CLI with prompts for vars:  ./run-prover.sh

# If the env var PROVER_PRIVATE_KEY is not set, prompt for it
if [ -z "${PROVER_PRIVATE_KEY}" ]
then
  read -r -p "Enter the Aleo Prover account private key: "
  PROVER_PRIVATE_KEY=$REPLY
fi

if [ "${PROVER_PRIVATE_KEY}" == "" ]
then
  echo "Missing account private key. (run 'snarkos account new' and try again)"
  exit
fi

COMMAND="cargo run --release -- start --nodisplay --prover --private-key ${PROVER_PRIVATE_KEY}"

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
  STATUS=$(git pull)

  if [ "$STATUS" != "Already up to date." ]; then
    echo "Updated code found, rebuilding and relaunching prover"
    cargo clean
    kill -INT $!; sleep 2; $COMMAND &
  fi

  sleep 1800;
done
