#!/bin/bash

# Read the EC2 instance name from the user
read -p "Enter the EC2 instance name to filter by (e.g. Name) (default: devnet): " INSTANCE_NAME
INSTANCE_NAME="${INSTANCE_NAME:-devnet}"

# Read the PEM file path from the user or use the default in ~/.ssh
read -p "Enter the PEM file path (default: ~/.ssh/s3-devnet.pem): " PEM_FILE
PEM_FILE="${PEM_FILE:-~/.ssh/s3-devnet.pem}"

# Use the AWS CLI to describe running EC2 instances, filter by the provided name, and store the JSON output in a variable
instance_info=$(aws ec2 describe-instances \
  --filters "Name=tag:Name,Values=$INSTANCE_NAME" "Name=instance-state-name,Values=running" \
  --output json)

# Parse the JSON output to extract information about the instances
instance_ids=($(echo "$instance_info" | jq -r '.Reservations[].Instances[].InstanceId'))
instance_names=($(echo "$instance_info" | jq -r '.Reservations[].Instances[].Tags[] | select(.Key=="Name") | .Value'))
instance_states=($(echo "$instance_info" | jq -r '.Reservations[].Instances[].State.Name'))
instance_ips=($(echo "$instance_info" | jq -r '.Reservations[].Instances[].PublicIpAddress'))

# Initialize the SSH config string
SSH_CONFIG=""

# Loop through the instance IDs and print information for each instance
for i in ${!instance_ids[@]}; do
#    echo "Instance ID: ${instance_ids[$i]}"
#    echo "Instance Name: ${instance_names[$i]}"
#    echo "Instance State: ${instance_states[$i]}"
#    echo "Instance IP: ${instance_ips[$i]}"
#    echo "------------------------"

    # Append SSH config entries to the string
    SSH_CONFIG+="Host aws-n$i"$'\n'
    SSH_CONFIG+="  HostName ${instance_ips[$i]}"$'\n'
    SSH_CONFIG+="  User ubuntu"$'\n'
    SSH_CONFIG+="  IdentityFile $PEM_FILE"$'\n'
    SSH_CONFIG+="  Port 22"$'\n'
    SSH_CONFIG+=$'\n'
done

# Print or save the SSH config string as needed
echo -e "\n\n# AWS Devnet Nodes\n\n$SSH_CONFIG"
