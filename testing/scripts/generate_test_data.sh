# This script will run the test data generation program in the `examples` folder and move the resulting file
# to their respective folders under the `src` directory.

# Generate test_data

cargo run --release --example test_data

mv test_data ../src/consensus
