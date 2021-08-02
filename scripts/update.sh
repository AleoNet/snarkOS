# Ensure we're on the right branch.
git checkout groth16 &> /dev/null

# Fetch the latest changes.
git pull

# Clean out existing database and cargo target.
rm -rf ~/.snarkOS &> /dev/null
rm Cargo.lock &> /dev/null
cargo clean

echo ""
echo "---------------------------------------------"
echo " Update succeeded, ready to restart snarkOS."
echo "---------------------------------------------"
echo ""
echo "cargo run --release"
echo ""
echo "---------------------------------------------"
