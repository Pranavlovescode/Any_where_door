#!/bin/bash

# Uninstall script for Anywhere Door Agent (Rust)
# Removes the service, binary, user account, and log directory

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if running as root
if [[ $EUID -ne 0 ]]; then
    echo -e "${RED}Error: This script must be run as root (use sudo)${NC}"
    echo "Usage: sudo ./scripts/uninstall.sh"
    exit 1
fi

SERVICE_NAME="anywhere-door-agent"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
BINARY_PATH="/usr/local/bin/anywhere_door_agent"
SERVICE_USER="anywhere-door"
SERVICE_GROUP="anywhere-door"
LOG_DIR="/var/log/anywhere-door-agent"
WORK_DIR="/var/lib/anywhere-door-agent"

echo -e "${YELLOW}=== Uninstalling Anywhere Door Agent ===${NC}\n"

# Step 1: Stop the service
if systemctl is-active --quiet ${SERVICE_NAME}; then
    echo -e "${YELLOW}[1/5]${NC} Stopping ${SERVICE_NAME} service..."
    systemctl stop ${SERVICE_NAME}
    echo -e "${GREEN}✓ Service stopped${NC}"
else
    echo -e "${YELLOW}[1/5]${NC} Service is not running, skipping stop..."
fi

# Step 2: Disable the service
if systemctl is-enabled ${SERVICE_NAME} 2>/dev/null; then
    echo -e "${YELLOW}[2/5]${NC} Disabling ${SERVICE_NAME} service..."
    systemctl disable ${SERVICE_NAME}
    echo -e "${GREEN}✓ Service disabled${NC}"
else
    echo -e "${YELLOW}[2/5]${NC} Service is not enabled, skipping disable..."
fi

# Step 3: Remove systemd unit file
if [[ -f "${SERVICE_FILE}" ]]; then
    echo -e "${YELLOW}[3/5]${NC} Removing systemd unit file..."
    rm "${SERVICE_FILE}"
    systemctl daemon-reload
    echo -e "${GREEN}✓ Systemd unit file removed and daemon reloaded${NC}"
else
    echo -e "${YELLOW}[3/5]${NC} Systemd unit file not found, skipping..."
fi

# Step 4: Remove binary
if [[ -f "${BINARY_PATH}" ]]; then
    echo -e "${YELLOW}[4/5]${NC} Removing binary from ${BINARY_PATH}..."
    rm "${BINARY_PATH}"
    echo -e "${GREEN}✓ Binary removed${NC}"
else
    echo -e "${YELLOW}[4/5]${NC} Binary not found at ${BINARY_PATH}, skipping..."
fi

# Step 5: Remove service user and directories
echo -e "${YELLOW}[5/5]${NC} Cleaning up user account and directories..."

# Remove service user if it exists
if id "${SERVICE_USER}" &>/dev/null; then
    echo "  Removing user '${SERVICE_USER}'..."
    userdel "${SERVICE_USER}" 2>/dev/null || true
    echo -e "  ${GREEN}✓ User removed${NC}"
else
    echo "  User '${SERVICE_USER}' does not exist, skipping..."
fi

# Remove log directory
if [[ -d "${LOG_DIR}" ]]; then
    echo "  Removing log directory ${LOG_DIR}..."
    rm -rf "${LOG_DIR}"
    echo -e "  ${GREEN}✓ Log directory removed${NC}"
else
    echo "  Log directory not found, skipping..."
fi

# Remove working directory
if [[ -d "${WORK_DIR}" ]]; then
    echo "  Removing working directory ${WORK_DIR}..."
    rm -rf "${WORK_DIR}"
    echo -e "  ${GREEN}✓ Working directory removed${NC}"
else
    echo "  Working directory not found, skipping..."
fi

echo ""
echo -e "${GREEN}=== Uninstall Complete ===${NC}"
echo "The Anywhere Door Agent has been successfully uninstalled."
echo ""
echo -e "${YELLOW}Removed:${NC}"
echo "  ✓ Systemd service (${SERVICE_NAME})"
echo "  ✓ Binary (${BINARY_PATH})"
echo "  ✓ Service user (${SERVICE_USER})"
echo "  ✓ Log directory (${LOG_DIR})"
echo "  ✓ Working directory (${WORK_DIR})"
