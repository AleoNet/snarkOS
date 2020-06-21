# This script generates snarkOS parameters, the genesis block, and test data.
# Results are stored in their respective folders in `snarkos-parameters` and `snarkos-testing`

printf "\nParameter generation starting...\n\n"

# Generate parameters

cd parameters/scripts || printf "\nError - cannot find 'parameters/scripts' folder\n\n"
./parameters_setup.sh

# Generate genesis block

./genesis_block_setup.sh

# Generate test data

cd ../../testing/scripts || printf "\nError - cannot find 'testing/scripts' folder\n\n"
./generate_test_data.sh

printf "\nParameter generation completed.\n\n"
