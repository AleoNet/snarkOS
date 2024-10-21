#!/bin/bash
# USAGE examples:
  # CLI with env vars: VALIDATOR_PRIVATE_KEY=APrivateKey1... PEERS=core_client_ip_1:4130,core_client_ip_2:4130,... VALIDATORS=validator_ip_1:5000,validator_ip_2:5000,... ./run-validator.sh
  # CLI with prompts for vars:  ./run-validator.sh

# If the env var VALIDATOR_PRIVATE_KEY is not set, prompt for it
if [ -z "${VALIDATOR_PRIVATE_KEY}" ]
then
  read -r -p "Enter the Aleo Validator account private key: "
  VALIDATOR_PRIVATE_KEY=$REPLY
fi

if [ "${VALIDATOR_PRIVATE_KEY}" == "" ]
then
  echo "Missing account private key. (run 'snarkos account new' and try again)"
  exit
fi

# If the env var PEERS is not set, prompt for it
if [ -z "${PEERS}" ]
then
  read -r -p "Enter the peers (comma-separated) (e.g., validator_ip_1:4130,validator_ip_2:4130,...,core_client_ip_1:4130,core_client_ip_2:4130,...): "
  PEERS=$REPLY
fi

if [ "${PEERS}" == "" ]
then
  echo "Missing peers."
  exit 1
fi

# If the env var VALIDATORS is not set, prompt for it
if [ -z "${VALIDATORS}" ]
then
  read -r -p "Enter the validators (comma-separated) (e.g., validator_ip_1:5000,validator_ip_2:5000,...): "
  VALIDATORS=$REPLY
fi

if [ "${VALIDATORS}" == "" ]
then
  echo "Missing validators."
  exit 1
fi

COMMAND="cargo run --release -- start --nodisplay --validator --bft 0.0.0.0:5000 --node 0.0.0.0:4130 --peers ${PEERS} --validators ${VALIDATORS} --norest --private-key ${VALIDATOR_PRIVATE_KEY}"

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

echo "Checking for updates..."
git stash
rm Cargo.lock
STATUS=$(git pull)

if [ "$STATUS" != "Already up to date." ]; then
  echo "Updated code found, cleaning the project"
  cargo clean
fi

echo "Running an Aleo Validator node..."
$COMMAND &
wait
