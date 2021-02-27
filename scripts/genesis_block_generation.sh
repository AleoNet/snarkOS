# This script generates a genesis block for snarkOS.
# Results are stored in  in `snarkos-parameters`.

printf "\Genesis block generation starting...\n\n"

# Generate genesis block

cd parameters/scripts || printf "\nError - cannot find 'parameters/scripts' folder\n\n"
./genesis_block_setup.sh.sh

printf "\Genesis block generation completed.\n\n"