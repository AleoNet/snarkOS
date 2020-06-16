# This script will run the scripts to generate the snarkOS parameters, genesis block, and test data
# to be stored in their respective folders in `snarkos-parameters` and `snarkos-testing`

# Generate parameters

cd parameters/scripts

./parameters_setup.sh

# Generate genesis block

./genesis_block_setup.sh

cd ../../

# Generate test data

cd testing/scripts

./generate_test_data.sh
