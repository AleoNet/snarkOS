# This script will run the predicate SNARK setup and move the resulting `.params`
# and `.checksum` files to `params` folder under the `src` directory.
# If the parameter size has changed, you will need to manually update these in each corresponding struct.

cargo run --release --example predicate_snark

mv predicate_snark_pk.params ../src/params
mv predicate_snark_pk.checksum ../src/params

mv predicate_snark_vk.params ../src/params
mv predicate_snark_vk.checksum ../src/params