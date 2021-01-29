cd /home/circleci/project/ &&
rm -rf target/debug/base_dpc* && rm -rf target/debug/consensus_dpc* && rm -rf target/debug/consensus_integration* && rm -rf target/debug/miner* && rm -rf target/debug/protected_rpc_tests*
ls target
echo "-1-"
ls target/debug
echo "-2-"
ls target/debug/deps
echo "-3-"
for file in target/debug/*-*[^\.d];
  do
    mkdir -p "target/cov/$(basename $file)";
    echo "Processing target/cov/$(basename $file)"
    /usr/local/bin/kcov --exclude-pattern=/.cargo,/usr/lib --verify "target/cov/$(basename $file)" "$file";
  done
