#!/bin/bash
# USAGE examples:
  # CLI with env vars: VALIDATOR_PRIVATE_KEY=APrivateKey1...  ./run-validator.sh
  # CLI with prompts for vars:  ./run-validator.sh

# If the env var VALIDATOR_PRIVATE_KEY is not set, prompt for it
if [ -z "${VALIDATOR_PRIVATE_KEY}" ]
then
  read -r -p "Enter the Aleo Validator account private key: "
  VALIDATOR_PRIVATE_KEY=$REPLY
fi

if [ "${VALIDATOR_PRIVATE_KEY}" == "" ]
then
  VALIDATOR_PRIVATE_KEY="APrivateKey1zkp8cC4jgHEBnbtu3xxs1Ndja2EMizcvTRDq5Nikdkukg1p"
fi

COMMAND="cargo run --release -- start --nodisplay --validator ${VALIDATOR_PRIVATE_KEY}"

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

echo "Running an Aleo Validator node..."
$COMMAND &

while :
do
  echo "Checking for updates..."
  git stash
  rm Cargo.lock
  STATUS=$(git pull)

  if [ "$STATUS" != "Already up to date." ]; then
    echo "Updated code found, rebuilding and relaunching validator"
    cargo clean
    kill -INT $!; sleep 2; $COMMAND &
  fi

  sleep 1800;
done
