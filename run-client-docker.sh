#!/bin/bash
# running the Aleo Testnet3 client with Docker Compose

# USAGE examples: 
  # CLI :  ./run-client-docker.sh

while :
do
  echo "Checking for the latest snarkOS container..."
  $(docker-compose pull && docker-compose --project-name aleo up -d)
  # sleeping 30 minutes
  sleep 1800
done
