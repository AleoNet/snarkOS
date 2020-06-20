# This script generates snarkOS parameters, the genesis block, and test data.
# Results are stored in their respective folders in `snarkos-parameters` and `snarkos-testing`

printf "\nParameter generation starting...\n\n"

# Generate parameters

../parameters/scripts/parameters_setup.sh

# Generate genesis block

../parameters/scripts/genesis_block_setup.sh

# Generate test data

../testing/scripts/generate_test_data.sh

printf "\nParameter generation completed.\n\n"
