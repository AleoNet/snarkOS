# This script will run the PoSW SNARK setup and move the resulting `.params`
# and `.checksum` files to `params` folder under the `src` directory.
# If the parameter size has changed, you will need to manually update these in each corresponding struct.

cargo run --release --example posw_snark

mv posw_snark_pk*.params ../src/params
mv posw_snark_pk.checksum ../src/params

mv posw_snark_vk.params ../src/params
mv posw_snark_vk.checksum ../src/params
