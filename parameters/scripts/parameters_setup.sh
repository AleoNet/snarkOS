# This script will run the parameter setup programs in the `examples` folder and move the resulting `.params` files
# to their respective folders under the `src` directory.
# If the parameter size or checksum has changed, you will need to manually update these in each corresponding struct.

cargo run --example account_commitment
cargo run --example account_signature
cargo run --example inner_snark
cargo run --example local_data_commitment
cargo run --example outer_snark
cargo run --example predicate_snark
cargo run --example predicate_vk_crh
cargo run --example record_commitment
cargo run --example serial_number_nonce_crh
cargo run --example value_commitment

mv account_commitment.params ../src/account_commitment
mv account_signature.params ../src/account_signature
mv inner_snark.params ../src/inner_snark
mv local_data_commitment.params ../src/local_data_commitment
mv outer_snark.params ../src/outer_snark
mv predicate_snark.params ../src/predicate_snark
mv predicate_vk_crh.params ../src/predicate_vk_crh
mv record_commitment.params ../src/record_commitment
mv serial_number_nonce_crh.params ../src/serial_number_nonce_crh
mv value_commitment.params ../src/value_commitment
