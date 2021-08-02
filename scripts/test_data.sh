# This script generates snarkOS test data used in the `snarkos-testing` suite.
# Results are stored in `snarkos-testing`

printf "\Test data generation starting...\n\n"

# Generate test data

cd /testing/scripts || printf "\nError - cannot find 'testing/scripts' folder\n\n"
./test_data.sh

printf "\nTest data generation completed.\n\n"
