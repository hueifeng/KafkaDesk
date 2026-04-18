#!/usr/bin/env sh
set -eu

PORT=1420
LOG_FILE=/tmp/traceforge-tauri-dev.log

if lsof -nP -iTCP:$PORT -sTCP:LISTEN >/dev/null 2>&1; then
  echo "TraceForge already running at http://127.0.0.1:$PORT"
  exit 0
fi

npm run tauri:dev >"$LOG_FILE" 2>&1 &

echo "TraceForge starting at http://127.0.0.1:$PORT"
echo "log: $LOG_FILE"
