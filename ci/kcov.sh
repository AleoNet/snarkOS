cd /opt/kcov-source &&
rm -rf target/debug/base_dpc* && rm -rf target/debug/consensus_dpc* &&
for file in target/debug/*-*[^\.d];
  do mkdir -p "target/cov/$(basename $file)";
    /usr/local/bin/kcov --exclude-pattern=/.cargo,/usr/lib --verify "target/cov/$(basename $file)" "$file";
  done
