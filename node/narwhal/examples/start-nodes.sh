#!/bin/bash

# The mode - bft or narwhal
mode=$1

# Default number of nodes to spin up
default_num_nodes=4

# Command to run for each node
command="cargo +stable r --release --example simple_node"
path=$(pwd)

terminal_app=""

case "$TERM_PROGRAM" in
"iTerm.app")
	terminal_app="iTerm"
	;;
"Apple_Terminal")
	terminal_app="Terminal"
	;;
*)
	terminal_app="Unknown"
	;;
esac

# Get the number of nodes from the command-line argument
num_nodes=${1:-$default_num_nodes}

# Loop to open terminal windows and execute the command
for ((i = 0; i < num_nodes; i++)); do
	if [[ "$terminal_app" == "iTerm" ]]; then
		osascript -e "tell application \"$terminal_app\" to create window with default profile"
		sleep 0.5
		osascript -e "tell application \"$terminal_app\" to tell current window to tell current session to write text \"cd $path && $command $i $num_nodes\""
	elif [[ "$terminal_app" == "Terminal" ]]; then
		osascript -e "tell application \"$terminal_app\" to do script \"cd $path; $command $i $num_nodes\""
	else
		if ! command -v xterm &>/dev/null; then
			echo "xterm could not be found, please install it"
			exit
		fi
		xterm -e "cd $path; $command $mode $i $num_nodes" &
	fi
done
