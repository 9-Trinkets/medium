#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
TAURI_DIR="${PROJECT_DIR}/src-tauri"
LOG_DIR="${HOME}/.medium"
LOG_PATH="${LOG_DIR}/daemon.log"
SOCKET_PATH="/tmp/medium_ghost_default_cmd.sock"

echo "Building Medium frontend..."
cd "${PROJECT_DIR}"
npm run build

echo "Installing Medium binary..."
cd "${TAURI_DIR}"
cargo install --path . --force

echo "Stopping existing Medium daemon..."
daemon_pids="$(ps -axo pid=,command= | awk '/(^|\/)medium daemon($| )/ { print $1 }')"
for pid in ${daemon_pids}; do
  if [[ -n "${pid}" ]]; then
    kill "${pid}" 2>/dev/null || true
  fi
done

if [[ -n "${daemon_pids}" ]]; then
  for _ in {1..20}; do
    any_running=0
    for pid in ${daemon_pids}; do
      if ps -p "${pid}" >/dev/null 2>&1; then
        any_running=1
        break
      fi
    done
    if [[ "${any_running}" -eq 0 ]]; then
      break
    fi
    sleep 0.2
  done
fi

if [[ -S "${SOCKET_PATH}" ]]; then
  rm -f "${SOCKET_PATH}"
fi

mkdir -p "${LOG_DIR}"
touch "${LOG_PATH}"

echo "Starting Medium daemon..."
nohup "${HOME}/.cargo/bin/medium" daemon >>"${LOG_PATH}" 2>&1 &

sleep 1
echo "Medium redeployed."
"${HOME}/.cargo/bin/medium" status
