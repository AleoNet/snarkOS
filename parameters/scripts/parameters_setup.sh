# This script will run the parameter setup programs in the `examples` folder and move the resulting `.params`
# and `.checksum` files to `params` folder under the `src` directory.
# If the parameter size has changed, you will need to manually update these in each corresponding struct.

cargo run --release --example account_commitment
cargo run --release --example account_signature
cargo run --release --example ledger_merkle_tree
cargo run --release --example local_data_crh
cargo run --release --example local_data_commitment
cargo run --release --example predicate_vk_crh
cargo run --release --example record_commitment
cargo run --release --example serial_number_nonce_crh
cargo run --release --example value_commitment

mv account_commitment.params ../src/params
mv account_commitment.checksum ../src/params

mv account_signature.params ../src/params
mv account_signature.checksum ../src/params

mv ledger_merkle_tree.params ../src/params
mv ledger_merkle_tree.checksum ../src/params

mv local_data_crh.params ../src/params
mv local_data_crh.checksum ../src/params

mv local_data_commitment.params ../src/params
mv local_data_commitment.checksum ../src/params

mv predicate_vk_crh.params ../src/params
mv predicate_vk_crh.checksum ../src/params

mv record_commitment.params ../src/params
mv record_commitment.checksum ../src/params

mv serial_number_nonce_crh.params ../src/params
mv serial_number_nonce_crh.checksum ../src/params

mv value_commitment.params ../src/params
mv value_commitment.checksum ../src/params

./predicate_snark.sh

./inner_snark.sh

./outer_snark.sh

./posw_snark.sh

./genesis_block_setup.sh
