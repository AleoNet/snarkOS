#!/bin/bash

SCRIPTNAME="miner.sh"
ARGS="$@"
BRANCH=$(git rev-parse --abbrev-ref HEAD)

self_update() {
    git fetch

    [ -n $(git diff --name-only origin/$BRANCH | grep $SCRIPTNAME) ] && {
        echo "Found a new version of the miner script"
        git pull --force
        git checkout $BRANCH
        git pull --force
        echo "Running the new version..."
        ./miner.sh $ARGS

        # Now exit this old instance
        exit 1
    }
    echo "Already the latest version."
}

self_update
