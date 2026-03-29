#!/usr/bin/env bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

BIN_SRC="$REPO_DIR/target/release/anywhere_door_agent"
UNIT_SRC="$REPO_DIR/deploy/linux/anywhere-door-agent.service"
BIN_DST="/usr/local/bin/anywhere_door_agent"
UNIT_DST="/etc/systemd/system/anywhere-door-agent.service"
DATA_DIR="/var/lib/anywhere-door-agent"
LOG_DIR="/var/log/anywhere-door-agent"

# Get the actual user (handles both direct execution and sudo)
if [[ $EUID -eq 0 ]]; then
  CALLING_USER="${SUDO_USER:-root}"
  SUDO=""
else
  CALLING_USER="$USER"
  if ! command -v sudo >/dev/null 2>&1; then
    echo "Run as root or install sudo." >&2
    exit 1
  fi
  SUDO="sudo"
fi

# Get user's home directory and group
CALLING_USER_HOME="$(eval echo ~$CALLING_USER)"
CALLING_USER_GROUP="$(id -gn $CALLING_USER)"

# Service runs as the calling user (not a separate system user)
SERVICE_USER="$CALLING_USER"
SERVICE_GROUP="$CALLING_USER_GROUP"

# Determine watch roots: watch ONLY user's home directory
# This monitors all changes in the user's home directory and subdirectories
WATCH_ROOTS="$CALLING_USER_HOME"

# Allow override via environment variable (comma-separated on Linux)
if [[ -n "${ANYWHERE_DOOR_WATCH_ROOTS:-}" ]]; then
  WATCH_ROOTS="$ANYWHERE_DOOR_WATCH_ROOTS"
fi

echo -e "${YELLOW}=== Installing Anywhere Door Agent ===${NC}\n"
echo "Installing for user: $CALLING_USER"
echo "User home directory: $CALLING_USER_HOME"
echo "User group: $CALLING_USER_GROUP"
echo ""

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

# Step 1: Verify calling user exists
echo -e "${YELLOW}[1/5]${NC} Verifying user '$SERVICE_USER'..."
if id "$SERVICE_USER" >/dev/null 2>&1; then
  echo -e "${GREEN}✓ User '$SERVICE_USER' verified${NC}"
else
  echo "Error: User '$SERVICE_USER' does not exist" >&2
  exit 1
fi

# Step 2: Skip - service runs as existing user, no group changes needed
echo -e "${YELLOW}[2/5]${NC} User setup..."
echo -e "${GREEN}✓ Service will run as user: $SERVICE_USER (group: $SERVICE_GROUP)${NC}"

# Step 3: Fix directory permissions for user's home
echo -e "${YELLOW}[3/5]${NC} Configuring directory permissions..."
$SUDO chmod g+rx "$CALLING_USER_HOME"
echo "  Applied: chmod g+rx $CALLING_USER_HOME"
if [[ -d "$CALLING_USER_HOME/Any_where_door" ]]; then
  $SUDO chmod -R g+rx "$CALLING_USER_HOME/Any_where_door"
  echo "  Applied: chmod -R g+rx $CALLING_USER_HOME/Any_where_door"
fi
echo -e "${GREEN}✓ Directory permissions configured${NC}"

# Step 4: Create service directories and install files
echo -e "${YELLOW}[4/5]${NC} Creating service directories and installing files..."
$SUDO mkdir -p "$DATA_DIR" "$LOG_DIR"
$SUDO chown -R "$SERVICE_USER:$SERVICE_GROUP" "$DATA_DIR" "$LOG_DIR"
$SUDO install -m 0755 "$BIN_SRC" "$BIN_DST"
echo -e "${GREEN}✓ Directories created and binary installed${NC}"

# Step 5: Create and install systemd unit with user-specific watch path
echo -e "${YELLOW}[5/5]${NC} Configuring systemd unit..."
$SUDO bash -c "cat > $UNIT_DST" << EOF
[Unit]
Description=Anywhere Door Agent - OS Level File Watcher (Rust)
After=network.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=$BIN_DST
WorkingDirectory=$DATA_DIR

# File watcher only - memory efficient event streaming
Environment=ANYWHERE_DOOR_ENABLE_OS_WATCHER=true
Environment=ANYWHERE_DOOR_FILE_EVENT_METADATA_OUTPUT=$LOG_DIR/file_event_metadata.ndjson
Environment=ANYWHERE_DOOR_WATCH_ROOTS=$WATCH_ROOTS

Restart=always
RestartSec=3
User=$SERVICE_USER
Group=$SERVICE_GROUP
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
EOF
echo -e "${GREEN}✓ Systemd unit configured (watching: $WATCH_ROOTS)${NC}"

$SUDO systemctl daemon-reload
$SUDO systemctl enable --now anywhere-door-agent.service

echo ""
echo -e "${GREEN}=== Installation Complete ===${NC}"
echo ""
echo -e "${YELLOW}Configuration:${NC}"
echo "  Service user: $SERVICE_USER"
echo "  Service group: $SERVICE_GROUP"
echo "  Binary location: $BIN_DST"
echo "  Log location: $LOG_DIR/file_event_metadata.ndjson"
echo "  Watching directory: $WATCH_ROOTS"
echo ""
echo -e "${YELLOW}How it works:${NC}"
echo "  • Service runs as: $SERVICE_USER (whoever runs the installer)"
echo "  • Watches: $WATCH_ROOTS"
echo "  • Output: NDJSON format with file events (create, modify, delete, etc.)"
echo ""
echo -e "${YELLOW}To watch additional directories:${NC}"
echo "  Modify /etc/systemd/system/anywhere-door-agent.service:"
echo "  Environment=ANYWHERE_DOOR_WATCH_ROOTS=/path1,/path2,/path3"
echo "  Then: sudo systemctl daemon-reload && sudo systemctl restart anywhere-door-agent.service"
echo ""
echo -e "${YELLOW}Verify installation:${NC}"
echo "  systemctl status anywhere-door-agent.service"
echo "  tail -f $LOG_DIR/file_event_metadata.ndjson"
echo ""
echo -e "${YELLOW}Current status:${NC}"
$SUDO systemctl status --no-pager --lines=10 anywhere-door-agent.service || true
