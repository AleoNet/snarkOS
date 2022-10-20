#!/bin/bash
> pids.txt

cargo build --release
for i in {1..50}
do
	./target/release/snarkos --connect_to_beacon 95.217.56.71:4130 --dev $((i)) --rest_port 
$((i + 11000)) --node 0.0.0.0:$((i+10000)) & echo "$!" >> pids.txt
done
