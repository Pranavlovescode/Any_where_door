#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

BIN_SRC="$REPO_DIR/target/release/anywhere_door_agent"
UNIT_SRC="$REPO_DIR/deploy/linux/anywhere-door-agent.service"
BIN_DST="/usr/local/bin/anywhere_door_agent"
UNIT_DST="/etc/systemd/system/anywhere-door-agent.service"
DATA_DIR="/var/lib/anywhere-door-agent"
LOG_DIR="/var/log/anywhere-door-agent"
SERVICE_USER="anywhere-door"

if ! command -v systemctl >/dev/null 2>&1; then
  echo "systemctl not found. This host does not appear to use systemd." >&2
  exit 1
fi

if [[ ! -f "$BIN_SRC" ]]; then
  echo "Missing release binary at $BIN_SRC" >&2
  echo "Build first with: cargo build --release" >&2
  exit 1
fi

if [[ ! -f "$UNIT_SRC" ]]; then
  echo "Missing unit file at $UNIT_SRC" >&2
  exit 1
fi

if [[ $EUID -eq 0 ]]; then
  SUDO=""
else
  if ! command -v sudo >/dev/null 2>&1; then
    echo "Run as root or install sudo." >&2
    exit 1
  fi
  SUDO="sudo"
fi

if ! id "$SERVICE_USER" >/dev/null 2>&1; then
  NOLOGIN_BIN="$(command -v nologin || true)"
  if [[ -z "$NOLOGIN_BIN" ]]; then
    NOLOGIN_BIN="/bin/false"
  fi
  $SUDO useradd --system --create-home --home-dir "$DATA_DIR" --shell "$NOLOGIN_BIN" "$SERVICE_USER"
fi

$SUDO mkdir -p "$DATA_DIR" "$LOG_DIR"
$SUDO chown -R "$SERVICE_USER:$SERVICE_USER" "$DATA_DIR" "$LOG_DIR"

$SUDO install -m 0755 "$BIN_SRC" "$BIN_DST"
$SUDO install -m 0644 "$UNIT_SRC" "$UNIT_DST"

$SUDO systemctl daemon-reload
$SUDO systemctl enable --now anywhere-door-agent.service

echo "Service installed and started. Current status:"
$SUDO systemctl status --no-pager --lines=20 anywhere-door-agent.service || true
