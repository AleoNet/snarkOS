#!/usr/bin/env bash

trap func exit
# Declare the function
function func() {
	kill $(jobs -p)
	echo "Done"
}

BEACON_COMMAND="cargo +stable run -- start --nodisplay --verbosity 0 --dev 0 --beacon"

echo "starting beacon as 0, check logs at ./beacon.log"
$BEACON_COMMAND '' >beacon.log 2>&1 &

# Start other validators without metrics
for i in 1 2 3 4; do
	# Enable metrics for first validator only
	extra_args=""
	if [[ "$i" -eq 1 ]]; then
		extra_args=" --metrics"
	fi
	VALIDATOR_COMMAND="cargo +stable run -- start$extra_args --nodisplay --verbosity 0 --dev ${i} --validator"
	echo "starting validator as $i, check logs at ./validator-$i.log"
	echo "command: $VALIDATOR_COMMAND"
	$VALIDATOR_COMMAND '' >validator-$i.log 2>&1 &
done

echo "All running, press Ctrl-C to stop"
wait
