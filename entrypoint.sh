#!/bin/bash
# running the Aleo Testnet3 client in a container

# check if environment variables are set
if [[ -z ${RUST_LOG+a} ]]; then
  RUST_LOG=debug
fi

if [[ -z ${SNARKOS_PORT+a} ]]; then
  SNARKOS_PORT="0.0.0.0:4133"
fi

if [[ -z ${RPC_PORT+a} ]]; then
  RPC_PORT="0.0.0.0:3033"
fi

if [[ -z ${LOGLEVEL+a} ]]; then
  LOGLEVEL="2"
fi

# if address is set
if [ -z ${ALEO_PRIVKEY+a} ]; then 
	/aleo/bin/snarkos --node ${SNARKOS_PORT} --rpc ${RPC_PORT} --verbosity ${LOGLEVEL} 
else
	/aleo/bin/snarkos --node ${SNARKOS_PORT} --rpc ${RPC_PORT} --verbosity ${LOGLEVEL} --private_key ${ALEO_PRIVKEY}
fi
