#!/bin/bash

if [ $RANDOM -lt 11000 ]
then

	# This script generates a transaction and broadcasts it to the network

	cd /home/ubuntu/snarkOS/toolkit/

	# Generate the transaction
	# cargo run --release --example dummy_transaction
	/home/ubuntu/snarkOS/target/release/examples/dummy_transaction

	token='ZjAyN2Y2ZjAtODNlOS00ZGUyLWJmNTAtMmUxZGM3ZTMwZDRi'
	transaction=$(cat /home/ubuntu/snarkOS/toolkit/dummy_transaction.txt)

	# Send a post request to aleo.network

	curl --location --request POST 'https://aleo-explorer-backend-prod.herokuapp.com/transaction/broadcast' \
	--header "Authorization: Bearer $token" \
	--header 'Content-Type: application/x-www-form-urlencoded' \
	--data-urlencode "transaction=$transaction"
else
	echo SKIPPED
fi
