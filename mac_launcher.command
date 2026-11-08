#!/bin/sh
cd "$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
sh scripts/launch.sh "$@"
result=$?
if [ "$result" -ne 0 ]; then
    printf '\nLaunch failed. Press Enter to close.'
    read -r unused
fi
exit "$result"
