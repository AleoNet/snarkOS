# This script will run the parameter setup programs in the `examples` folder and move the resulting `.params`
# and `.checksum` files to their respective folders under the `src` directory.
# If the parameter size has changed, you will need to manually update these in each corresponding struct.

cargo run --release --example account_commitment
cargo run --release --example account_signature
cargo run --release --example ledger_merkle_tree
cargo run --release --example local_data_commitment
cargo run --release --example predicate_vk_crh
cargo run --release --example record_commitment
cargo run --release --example serial_number_nonce_crh
cargo run --release --example value_commitment
cargo run --release --example posw

mv account_commitment.params ../src/account_commitment
mv account_commitment.checksum ../src/account_commitment

mv account_signature.params ../src/account_signature
mv account_signature.checksum ../src/account_signature

mv ledger_merkle_tree.params ../src/ledger_merkle_tree
mv ledger_merkle_tree.checksum ../src/ledger_merkle_tree

mv local_data_commitment.params ../src/local_data_commitment
mv local_data_commitment.checksum ../src/local_data_commitment

mv predicate_vk_crh.params ../src/predicate_vk_crh
mv predicate_vk_crh.checksum ../src/predicate_vk_crh

mv record_commitment.params ../src/record_commitment
mv record_commitment.checksum ../src/record_commitment

mv serial_number_nonce_crh.params ../src/serial_number_nonce_crh
mv serial_number_nonce_crh.checksum ../src/serial_number_nonce_crh

mv value_commitment.params ../src/value_commitment
mv value_commitment.checksum ../src/value_commitment

mv posw_pk.params ../src/posw
mv posw_vk.params ../src/posw

cargo run --release --example inner_snark
cargo run --release --example outer_snark
cargo run --release --example predicate_snark

mv inner_snark_pk.params ../src/inner_snark_pk
mv inner_snark_pk.checksum ../src/inner_snark_pk

mv inner_snark_vk.params ../src/inner_snark_vk
mv inner_snark_vk.checksum ../src/inner_snark_vk

mv outer_snark_pk.params ../src/outer_snark_pk
mv outer_snark_pk.checksum ../src/outer_snark_pk

mv outer_snark_vk.params ../src/outer_snark_vk
mv outer_snark_vk.checksum ../src/outer_snark_vk

mv predicate_snark_pk.params ../src/predicate_snark_pk
mv predicate_snark_pk.checksum ../src/predicate_snark_pk

mv predicate_snark_vk.params ../src/predicate_snark_vk
mv predicate_snark_vk.checksum ../src/predicate_snark_vk
