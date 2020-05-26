wget https://github.com/SimonKagstrom/kcov/archive/master.tar.gz &&
tar xzf master.tar.gz && cd kcov-master &&
mkdir build && cd build && cmake .. && make && sudo make install &&
cd ../.. && sudo rm -rf kcov-master && sudo rm -rf master.tar.gz &&
sudo rm -rf target/debug/base_dpc* && sudo rm -rf target/debug/consensus_dpc* &&
for file in target/debug/*-*[^\.d]; do mkdir -p "target/cov/$(basename $file)"; /usr/local/bin/kcov --exclude-pattern=/.cargo,/usr/lib --verify "target/cov/$(basename $file)" "$file"; done &&
bash <(curl -s https://codecov.io/bash) &&
echo "Uploaded code coverage"
