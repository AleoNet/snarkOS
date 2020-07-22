cd /home/circleci/project/ &&
rm -rf target/cov/base_dpc* && rm -rf target/cov/consensus_dpc* && rm -rf target/cov/consensus_integration && rm -rf target/cov/protected_rpc_tests*
for file in target/debug/*-*[^\.d];
  do
    mkdir -p "target/cov/$(basename $file)";
    echo "Processing target/cov/$(basename $file)"
    /usr/local/bin/kcov --exclude-pattern=/.cargo,/usr/lib --verify "target/cov/$(basename $file)" "$file";
  done
