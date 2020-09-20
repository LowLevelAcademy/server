#!/bin/bash

set -eu

timeout=${PLAYGROUND_TIMEOUT:-10}

# Don't use `exec` here. The shell is what prints out the useful
# "Killed" message
timeout --signal=KILL ${timeout} "$@"
