cd /opt/kcov-source/ && ls && for file in target/debug/*-*[^\.d];
  do
    echo $file;
    mkdir -p "target/cov/$(basename $file)";
    kcov --exclude-pattern=/.cargo,/usr/lib --verify "target/cov/$(basename $file)" "$file";
  done
