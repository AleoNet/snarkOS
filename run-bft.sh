#!/usr/bin/env bash

trap func exit
# Declare the function
function func() {
	kill $(jobs -p)
	echo "Done"
}

BEACON_COMMAND="cargo run -- start --nodisplay --verbosity 0 --dev 0 --beacon"

echo "starting beacon as 0, check logs at ./beacon.log"
$BEACON_COMMAND '' >beacon.log 2>&1 &

for i in 1 2 3 4; do
	VALIDATOR_COMMAND="cargo run -- start --nodisplay --verbosity 0 --dev ${i} --validator"
	echo "starting validator as $i, check logs at ./validator-$i.log"
	$VALIDATOR_COMMAND '' >validator-$i.log 2>&1 &
done

echo "All running, press Ctrl-C to stop"
wait
