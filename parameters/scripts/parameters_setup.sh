# This script will run the parameter setup programs in the `examples` folder and move the resulting `.params`
# and `.checksum` files to `params` folder under the `src` directory.
# If the parameter size has changed, you will need to manually update these in each corresponding struct.

cargo run --release --example account_commitment
cargo run --release --example account_encryption
cargo run --release --example account_signature
cargo run --release --example ledger_merkle_tree
cargo run --release --example local_data_crh
cargo run --release --example local_data_commitment
cargo run --release --example program_vk_crh
cargo run --release --example record_commitment
cargo run --release --example encrypted_record_crh
cargo run --release --example serial_number_nonce_crh

mv account_commitment.params ../src/params
mv account_commitment.checksum ../src/params

mv account_encryption.params ../src/params
mv account_encryption.checksum ../src/params

mv account_signature.params ../src/params
mv account_signature.checksum ../src/params

mv ledger_merkle_tree.params ../src/params
mv ledger_merkle_tree.checksum ../src/params

mv local_data_crh.params ../src/params
mv local_data_crh.checksum ../src/params

mv local_data_commitment.params ../src/params
mv local_data_commitment.checksum ../src/params

mv program_vk_crh.params ../src/params
mv program_vk_crh.checksum ../src/params

mv record_commitment.params ../src/params
mv record_commitment.checksum ../src/params

mv encrypted_record_crh.params ../src/params
mv encrypted_record_crh.checksum ../src/params

mv serial_number_nonce_crh.params ../src/params
mv serial_number_nonce_crh.checksum ../src/params

./noop_program_snark.sh

./inner_snark.sh

./outer_snark.sh

./posw_snark.sh

./genesis_block_setup.sh
