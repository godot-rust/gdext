#!/bin/bash
# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# Executes a command and retries it a few times with exponential backoff.

retryIntervals=(1 2 4) # seconds
limit=${#retryIntervals[@]}

for ((attempt=0; attempt<limit; attempt++)); do
    # Check the exit code of the command
#    if ./rand_success.sh; then
    if "$@"; then
        # Command succeeded, exit the loop
        echo "Done."
        exit 0
    fi

    # Calculate the sleep duration using the retry interval from the array
    sleepDuration=${retryIntervals[$attempt]}

    # Sleep for the calculated duration
    echo "Failed command '$1'."
    if [[ $attempt -ne $((limit - 1)) ]]; then
        echo "Retry #$attempt in $sleepDuration seconds..."
        sleep "$sleepDuration"
    fi
done

echo "::error::Failed after $limit attempts."
exit 1
