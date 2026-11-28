#!/usr/bin/env bash
# Cargo target runner: execute the actual iOS test binary, not a host test.
set -euo pipefail
: "${RPCORE_SIMULATOR_UDID:?Select and boot an iOS simulator first}"
binary="$1"
shift
case "$binary" in
  /*) ;;
  *) binary="$PWD/$binary" ;;
esac
exec xcrun simctl spawn "$RPCORE_SIMULATOR_UDID" "$binary" "$@"
