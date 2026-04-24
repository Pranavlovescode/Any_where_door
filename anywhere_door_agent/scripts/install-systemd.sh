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

# ============================================================================
# STEP 0: AUTHENTICATION & DEVICE REGISTRATION
# ============================================================================

CREDENTIALS_FILE="$CALLING_USER_HOME/.anywheredoor"
WATCH_CONFIG_FILE="$CALLING_USER_HOME/.anywheredoor_watch_roots"
SERVER_URL="${ANYWHERE_DOOR_SERVER_URL:-http://127.0.0.1:8000}"

# Check if credentials already exist
if [[ -f "$CREDENTIALS_FILE" ]]; then
  echo -e "${GREEN}✓ Device credentials found${NC}"
  # Load watch roots from config if it exists
  if [[ -f "$WATCH_CONFIG_FILE" ]]; then
    WATCH_ROOTS=$(grep -o '"watch_roots":"[^"]*"' "$WATCH_CONFIG_FILE" | cut -d'"' -f4)
    echo -e "${GREEN}✓ Watch configuration loaded: $WATCH_ROOTS${NC}"
  fi
else
  # First-time setup: authentication required
  echo -e "${YELLOW}=== First-Time Setup: User Authentication ===${NC}\n"
  echo "This service requires authentication to register this device."
  echo ""
  
  # Prompt for credentials
  read -p "Enter username: " USERNAME
  read -sp "Enter password: " PASSWORD
  echo ""
  
  # Attempt login
  echo ""
  echo -e "${YELLOW}Authenticating with server at: $SERVER_URL${NC}"
  echo -e "${YELLOW}Sending login request...${NC}"
  
  # Remove spaces and capture response
  LOGIN_RESPONSE=$(curl -s --max-time 10 -X POST "$SERVER_URL/auth/login" \
    -H "Content-Type: application/json" \
    -d "{\"username\": \"$USERNAME\", \"password\": \"$PASSWORD\"}")
  
  # Extract JWT from response - multiple strategies for robustness
  JWT=$(echo "$LOGIN_RESPONSE" | sed -n 's/.*"jwt":"\([^"]*\)".*/\1/p')
  
  # If JWT extraction failed, try alternative method
  if [[ -z "$JWT" ]]; then
    JWT=$(echo "$LOGIN_RESPONSE" | grep -o '"jwt":"[^"]*"' | cut -d'"' -f4)
  fi
  
  # Extract error message if present
  ERROR=$(echo "$LOGIN_RESPONSE" | sed -n 's/.*"detail":"\([^"]*\)".*/\1/p')
  
  if [[ -z "$JWT" ]]; then
    echo -e "${RED}✗ Authentication failed${NC}"
    if [[ -n "$ERROR" ]]; then
      echo "Error: $ERROR"
    else
      echo "Could not connect to server at: $SERVER_URL"
      echo "Ensure the backend server is running at: $SERVER_URL"
      echo ""
      echo "Full response:"
      echo "$LOGIN_RESPONSE"
    fi
    exit 1
  fi
  
  echo -e "${GREEN}✓ Authentication successful${NC}"
  
  # Device registration with JWT
  echo -e "${YELLOW}Registering device...${NC}"
  
  # Get hostname - use uname -n which is more universally available
  HOSTNAME=$(whoami)
  
  REGISTER_RESPONSE=$(curl -s --max-time 10 -X POST "$SERVER_URL/auth/register-device" \
    -H "Content-Type: application/json" \
    -d "{\"device_name\": \"$(whoami)@$HOSTNAME\", \"jwt\": \"$JWT\"}")
  
  # Extract device credentials - multiple strategies for robustness
  DEVICE_ID=$(echo "$REGISTER_RESPONSE" | sed -n 's/.*"device_id":"\([^"]*\)".*/\1/p')
  if [[ -z "$DEVICE_ID" ]]; then
    DEVICE_ID=$(echo "$REGISTER_RESPONSE" | grep -o '"device_id":"[^"]*"' | cut -d'"' -f4)
  fi
  
  DEVICE_SECRET=$(echo "$REGISTER_RESPONSE" | sed -n 's/.*"device_secret":"\([^"]*\)".*/\1/p')
  if [[ -z "$DEVICE_SECRET" ]]; then
    DEVICE_SECRET=$(echo "$REGISTER_RESPONSE" | grep -o '"device_secret":"[^"]*"' | cut -d'"' -f4)
  fi
  
  if [[ -z "$DEVICE_ID" ]] || [[ -z "$DEVICE_SECRET" ]]; then
    echo -e "${RED}✗ Device registration failed${NC}"
    echo "Response: $REGISTER_RESPONSE"
    exit 1
  fi
  
  echo -e "${GREEN}✓ Device registered (ID: ${DEVICE_ID:0:8}...)${NC}"
  
  # Save credentials
  TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%S.%3N")
  mkdir -p "$(dirname "$CREDENTIALS_FILE")"
  cat > "$CREDENTIALS_FILE" << CREDS_EOF
{
  "device_id": "$DEVICE_ID",
  "device_secret": "$DEVICE_SECRET",
  "username": "$USERNAME",
  "password": "$PASSWORD",
  "jwt": "$JWT",
  "timestamp": "$TIMESTAMP"
}
CREDS_EOF
  chmod 600 "$CREDENTIALS_FILE"
  echo -e "${GREEN}✓ Credentials saved to: $CREDENTIALS_FILE${NC}"
fi

# ============================================================================
# STEP 1: DIRECTORY SELECTION
# ============================================================================

# Determine watch roots with interactive selection (unless pre-configured)
if [[ -z "${WATCH_ROOTS:-}" ]] && [[ -z "${ANYWHERE_DOOR_WATCH_ROOTS:-}" ]]; then
  # Interactive directory selection
  echo ""
  echo -e "${YELLOW}=== Directory Selection ===${NC}\n"
  echo "Choose which directories to watch:"
  echo "[1] Home directory ($CALLING_USER_HOME) - Default"
  echo "[2] Entire filesystem (/)"
  echo "[3] Custom directories (enter paths separated by comma)"
  echo "[4] Skip and configure manually later"
  echo ""
  read -p "Enter choice (1-4): " dir_choice
  
  case "$dir_choice" in
    1)
      WATCH_ROOTS="$CALLING_USER_HOME"
      echo -e "${GREEN}Selected: $WATCH_ROOTS${NC}"
      ;;
    2)
      WATCH_ROOTS="/"
      echo -e "${GREEN}Selected: / (entire filesystem)${NC}"
      ;;
    3)
      echo ""
      echo "Enter directories to watch (separated by comma):"
      echo "Example: /home/user,/var/log,/opt/data"
      read -p "Directories: " WATCH_ROOTS
      echo -e "${GREEN}Selected: $WATCH_ROOTS${NC}"
      ;;
    4)
      WATCH_ROOTS="$CALLING_USER_HOME"
      echo -e "${YELLOW}Will use default home directory, but can be changed later${NC}"
      ;;
    *)
      WATCH_ROOTS="$CALLING_USER_HOME"
      echo -e "${YELLOW}Invalid choice. Using default: $CALLING_USER_HOME${NC}"
      ;;
  esac
  
  # Save watch configuration
  TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%S.%3NZ")
  cat > "$WATCH_CONFIG_FILE" << CONFIG_EOF
{
  "watch_roots": "$WATCH_ROOTS",
  "created_at": "$TIMESTAMP"
}
CONFIG_EOF
  chmod 600 "$WATCH_CONFIG_FILE"
  echo -e "${GREEN}✓ Watch config saved to: $WATCH_CONFIG_FILE${NC}"
elif [[ -n "${ANYWHERE_DOOR_WATCH_ROOTS:-}" ]]; then
  # Use environment variable if provided
  WATCH_ROOTS="$ANYWHERE_DOOR_WATCH_ROOTS"
  echo -e "${YELLOW}Using configured watch roots: $WATCH_ROOTS${NC}"
fi

echo ""
echo -e "${YELLOW}=== Installing Anywhere Door Agent ===${NC}\n"
echo "Installing for user: $CALLING_USER"
echo "User home directory: $CALLING_USER_HOME"
echo "User group: $CALLING_USER_GROUP"
echo "Watching: $WATCH_ROOTS"
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
echo -e "${YELLOW}[1/6]${NC} Verifying user '$SERVICE_USER'..."
if id "$SERVICE_USER" >/dev/null 2>&1; then
  echo -e "${GREEN}✓ User '$SERVICE_USER' verified${NC}"
else
  echo "Error: User '$SERVICE_USER' does not exist" >&2
  exit 1
fi

# Step 2: Skip - service runs as existing user, no group changes needed
echo -e "${YELLOW}[2/6]${NC} User setup..."
echo -e "${GREEN}✓ Service will run as user: $SERVICE_USER (group: $SERVICE_GROUP)${NC}"

# Step 3: Fix directory permissions for user's home
echo -e "${YELLOW}[3/6]${NC} Configuring directory permissions..."
$SUDO chmod g+rx "$CALLING_USER_HOME"
echo "  Applied: chmod g+rx $CALLING_USER_HOME"
if [[ -d "$CALLING_USER_HOME/Any_where_door" ]]; then
  $SUDO chmod -R g+rx "$CALLING_USER_HOME/Any_where_door"
  echo "  Applied: chmod -R g+rx $CALLING_USER_HOME/Any_where_door"
fi
echo -e "${GREEN}✓ Directory permissions configured${NC}"

# Step 4: Create service directories and install files
echo -e "${YELLOW}[4/6]${NC} Creating service directories and installing files..."
$SUDO mkdir -p "$DATA_DIR" "$LOG_DIR"
$SUDO chown -R "$SERVICE_USER:$SERVICE_GROUP" "$DATA_DIR" "$LOG_DIR"
$SUDO install -m 0755 "$BIN_SRC" "$BIN_DST"
echo -e "${GREEN}✓ Directories created and binary installed${NC}"

# Step 5: Copy credential files to appropriate location if needed
echo -e "${YELLOW}[5/6]${NC} Setting up credentials..."
if [[ -f "$CREDENTIALS_FILE" ]]; then
  echo -e "${GREEN}✓ Device credentials ready${NC}"
else
  echo -e "${RED}✗ Device credentials not found${NC}"
  echo "This should have been created during authentication step"
  exit 1
fi
echo -e "${GREEN}✓ Credentials configured${NC}"

# Step 6: Create and install systemd unit with user-specific watch path
echo -e "${YELLOW}[6/6]${NC} Configuring systemd unit..."
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
echo "  Credentials: $CREDENTIALS_FILE"
echo "  Watch config: $WATCH_CONFIG_FILE"
echo "  Log location: $LOG_DIR/file_event_metadata.ndjson"
echo "  Watching directories: $WATCH_ROOTS"
echo ""
echo -e "${YELLOW}Device Registration:${NC}"
echo "  ✓ User authentication completed"
echo "  ✓ Device registered with backend"
echo "  ✓ Credentials securely stored"
echo ""
echo -e "${YELLOW}How it works:${NC}"
echo "  • Service runs as: $SERVICE_USER"
echo "  • Watches: $WATCH_ROOTS"
echo "  • Output: NDJSON format with file events (create, modify, delete, etc.)"
echo ""
echo -e "${YELLOW}To watch additional directories:${NC}"
echo "  Modify /etc/systemd/system/anywhere-door-agent.service:"
echo "  Environment=ANYWHERE_DOOR_WATCH_ROOTS=/path1,/path2,/path3"
echo "  Then: sudo systemctl daemon-reload && sudo systemctl restart anywhere-door-agent.service"
echo ""
echo -e "${YELLOW}To reconfigure authentication or directories:${NC}"
echo "  1. Remove credentials: rm $CREDENTIALS_FILE $WATCH_CONFIG_FILE"
echo "  2. Run this installer again"
echo ""
echo -e "${YELLOW}Verify installation:${NC}"
echo "  systemctl status anywhere-door-agent.service"
echo "  tail -f $LOG_DIR/file_event_metadata.ndjson"
echo ""
echo -e "${YELLOW}Current status:${NC}"
$SUDO systemctl status --no-pager --lines=10 anywhere-door-agent.service || true
