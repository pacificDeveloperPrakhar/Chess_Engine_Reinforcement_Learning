#!/usr/bin/env bash
#
# build_and_log.sh
# Builds, tests, and (optionally) runs a Rust/Cargo project,
# logging all output to timestamped log files.
#
# Usage:
#   ./build_and_log.sh            # build + test only
#   ./build_and_log.sh --run      # build + test + run
#
set -euo pipefail

# Directory to store logs (created if missing)
LOG_DIR="logs"
mkdir -p "$LOG_DIR"

# Timestamp so each run gets its own log file
TIMESTAMP="$(date +'%Y%m%d_%H%M%S')"

BUILD_LOG="${LOG_DIR}/build_${TIMESTAMP}.log"
TEST_LOG="${LOG_DIR}/test_${TIMESTAMP}.log"
RUN_LOG="${LOG_DIR}/run_${TIMESTAMP}.log"

RUN_AFTER_BUILD=false
if [[ "${1:-}" == "--run" ]]; then
    RUN_AFTER_BUILD=true
fi

echo "==> Building project..."
if cargo build > "$BUILD_LOG" 2>&1; then
    echo "    Build succeeded. Log: $BUILD_LOG"
else
    echo "    Build FAILED. See log: $BUILD_LOG"
    exit 1
fi

echo "==> Running tests..."
if cargo test > "$TEST_LOG" 2>&1; then
    echo "    Tests passed. Log: $TEST_LOG"
else
    echo "    Tests FAILED. See log: $TEST_LOG"
    exit 1
fi

if [[ "$RUN_AFTER_BUILD" == true ]]; then
    echo "==> Running project..."
    # Not using '2>&1' redirection alone here in case you want to
    # still see output live; tee lets you watch AND log at once.
    cargo run 2>&1 | tee "$RUN_LOG"
    echo "    Run finished. Log: $RUN_LOG"
fi

echo "==> Done. Logs are in the '$LOG_DIR' directory."